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
            "# Révisions\n\n"
            "- [ ] Relire le cours\n"
            "- [ ] Faire une fiche\n"
            "- [x] Ancien exercice\n",
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

            if not page.get_by_role("heading", name="À faire").is_visible():
                errors.append("la page des tâches ne s'affiche pas")
            if page.locator(".task-row").count() != 0:
                errors.append("l'aperçu démarre avec des tâches fictives")

            for width, height in VIEWPORTS:
                page.set_viewport_size({"width": width, "height": height})
                page.wait_for_timeout(100)
                if page.locator("body").evaluate("element => element.scrollWidth") > width:
                    errors.append(f"débordement horizontal à {width}px")
                if not page.get_by_role("heading", name="À faire").is_visible():
                    errors.append(f"titre absent à {width}px")

            page.set_viewport_size({"width": 390, "height": 844})
            page.set_input_files("input[type=file]", str(todo_path))
            page.wait_for_function("document.querySelectorAll('.task-row').length === 3")
            if page.locator(".task-row").count() != 3:
                errors.append("le chargement du TODO ne restitue pas les trois tâches")

            new_task = page.locator("#new-task")
            new_task.fill("Résoudre trois exercices")
            new_task.press("Enter")
            page.wait_for_function("document.querySelectorAll('.task-row').length === 4")
            if page.locator(".task-row").count() != 4:
                errors.append("l'ajout graphique n'ajoute pas la tâche")
            if page.evaluate("document.activeElement?.id") != "new-task":
                errors.append("le focus ne revient pas au champ après ajout")
            if "Aperçu web" not in page.get_by_role("status").inner_text():
                errors.append("le statut d'aperçu web n'est pas annoncé")

            first_checkbox = page.locator("input[type=checkbox]").first
            first_checkbox.check()
            if not first_checkbox.is_checked():
                errors.append("la coche graphique ne change pas l'état visible")

            page.get_by_role("button", name="Faire une fiche", exact=True).click()
            if not page.get_by_role("heading", name="Faire une fiche", exact=True).is_visible():
                errors.append("la page de détails ne s'ouvre pas")

            page.get_by_role("button", name="Terminer", exact=True).click()
            if not page.get_by_text("Terminée", exact=True).is_visible():
                errors.append("la modification du statut depuis les détails échoue")

            note = page.locator("#task-note")
            note.fill("À revoir avec un exemple concret")
            page.get_by_role("button", name="Retour aux tâches").click()
            page.get_by_role("button", name="Faire une fiche", exact=True).click()
            if note.input_value() != "À revoir avec un exemple concret":
                errors.append("la modification de la note n'est pas persistée localement")

            page.screenshot(
                path=str(Path(temp_dir) / "taskhud-browser-final.png"), full_page=True
            )
            browser.close()

    errors.extend(f"erreur navigateur: {error}" for error in page_errors)
    errors.extend(f"erreur console: {error}" for error in console_errors)
    print(
        json.dumps(
            {
                "pass": not errors,
                "browser_cases": [
                    "chargement TODO",
                    "ajout",
                    "cochage",
                    "détails",
                    "modification du statut",
                    "modification d'une note",
                    "responsive sans débordement",
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
