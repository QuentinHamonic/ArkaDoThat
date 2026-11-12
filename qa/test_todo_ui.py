import json
import tempfile
from pathlib import Path

from playwright.sync_api import sync_playwright


VIEWPORTS = [(320, 640), (390, 844), (768, 900), (1024, 900), (1440, 900)]


def main() -> None:
    errors: list[str] = []
    page_errors: list[str] = []
    console_errors: list[str] = []

    with tempfile.TemporaryDirectory(prefix="taskhud-qa-") as temp_dir:
        todo_path = Path(temp_dir) / "revision.md"
        todo_path.write_text(
            "# Revision\n\n"
            "- [ ] Reread the lesson\n"
            "- [ ] Make a summary sheet\n"
            "- [x] Old exercise\n",
            encoding="utf-8",
        )

        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(headless=True)
            page = browser.new_page(viewport={"width": 390, "height": 844})
            page.on("pageerror", lambda error: page_errors.append(str(error)))
            page.on(
                "console",
                lambda message: console_errors.append(message.text)
                if message.type == "error"
                else None,
            )
            page.goto("http://127.0.0.1:5173", wait_until="networkidle")

            if not page.get_by_role("heading", name="To do").is_visible():
                errors.append("the task page is not displayed")
            if page.locator(".task-row").count() != 0:
                errors.append("the preview starts with fake tasks")
            if page.locator("h1").inner_text() != "Tasks":
                errors.append("the default header title is not English")
            if page.evaluate("document.documentElement.lang") != "en":
                errors.append("the default document language is not en")

            for width, height in VIEWPORTS:
                page.set_viewport_size({"width": width, "height": height})
                page.wait_for_timeout(100)
                if page.locator("body").evaluate("element => element.scrollWidth") > width:
                    errors.append(f"horizontal overflow at {width}px")
                if not page.get_by_role("heading", name="To do").is_visible():
                    errors.append(f"heading missing at {width}px")

            page.set_viewport_size({"width": 390, "height": 844})
            page.set_input_files("input[type=file]", str(todo_path))
            page.wait_for_function("document.querySelectorAll('.task-row').length === 3")
            if page.locator(".task-row").count() != 3:
                errors.append("loading the TODO does not show the three tasks")

            new_task = page.locator("#new-task")
            new_task.fill("Solve three exercises")
            new_task.press("Enter")
            page.wait_for_function("document.querySelectorAll('.task-row').length === 4")
            if page.locator(".task-row").count() != 4:
                errors.append("adding from the UI does not add the task")
            if page.evaluate("document.activeElement?.id") != "new-task":
                errors.append("focus does not return to the field after adding")
            if "Web preview" not in page.get_by_role("status").inner_text():
                errors.append("the web preview status is not announced")

            first_checkbox = page.locator("input[type=checkbox]").first
            first_checkbox.check()
            if not first_checkbox.is_checked():
                errors.append("checking from the UI does not change the visible state")

            page.get_by_role("button", name="Make a summary sheet", exact=True).click()
            if not page.get_by_role("heading", name="Make a summary sheet", exact=True).is_visible():
                errors.append("the details page does not open")

            page.get_by_role("button", name="Complete", exact=True).click()
            if not page.get_by_text("Done", exact=True).is_visible():
                errors.append("changing the status from the details page fails")

            note = page.locator("#task-note")
            note.fill("Review with a concrete example")
            page.get_by_role("button", name="Back to tasks").click()
            page.get_by_role("button", name="Make a summary sheet", exact=True).click()
            if note.input_value() != "Review with a concrete example":
                errors.append("the note change is not kept locally")
            page.get_by_role("button", name="Back to tasks").click()

            # T9: switching the language applies at once, keeps the current list and survives a reload.
            page.get_by_role("button", name="Open menu").click()
            page.get_by_role("button", name="Settings", exact=True).click()
            page.get_by_label("Language").select_option("fr")
            if page.locator("h1").inner_text() != "Paramètres":
                errors.append("switching to French does not change the visible title")
            if page.evaluate("document.documentElement.lang") != "fr":
                errors.append("<html lang> does not follow the French choice")
            if page.get_by_role("button", name="Ouvrir le menu").count() != 1:
                errors.append("the menu accessible label is not translated")
            page.get_by_role("button", name="Retour aux tâches").click()
            if page.locator("h1").inner_text() != "Tâches":
                errors.append("the task page title is not translated")
            if page.locator(".task-row").count() != 4:
                errors.append("switching the language loses the current tasks")

            page.reload(wait_until="networkidle")
            if page.locator("h1").inner_text() != "Tâches":
                errors.append("the French choice does not survive a reload")
            if not page.get_by_role("heading", name="À faire").is_visible():
                errors.append("the task heading is not French after a reload")
            if page.evaluate("document.documentElement.lang") != "fr":
                errors.append("<html lang> is not fr after a reload")

            # Language-independent selectors, so a failed reload above is reported instead of timing out here.
            page.locator(".menu-button").click()
            page.locator(".app-menu button").first.click()
            page.locator("#language-select").select_option("en")
            page.reload(wait_until="networkidle")
            if page.locator("h1").inner_text() != "Tasks":
                errors.append("switching back to English does not survive a reload")

            page.screenshot(
                path=str(Path(temp_dir) / "taskhud-browser-final.png"), full_page=True
            )
            browser.close()

    errors.extend(f"browser error: {error}" for error in page_errors)
    errors.extend(f"console error: {error}" for error in console_errors)
    print(
        json.dumps(
            {
                "pass": not errors,
                "browser_cases": [
                    "load TODO",
                    "add",
                    "check",
                    "details",
                    "status change",
                    "note change",
                    "responsive without overflow",
                    "language switch and reload",
                ],
                "errors": errors,
            },
            ensure_ascii=False,
        )
    )
    if errors:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
