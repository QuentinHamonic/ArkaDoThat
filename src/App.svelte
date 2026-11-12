<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-dialog';
  import { appendTask, parseTodo, tasksMissingFrom, toggleTask } from './lib/todo';
  import { dictionaries, languages, loadLanguage, saveLanguage, type Language, type Messages } from './lib/i18n';

  type TodoTask = {
    id: string;
    title: string;
    checked: boolean;
    lineIndex: number;
  };

  type TodoSnapshot = {
    path: string;
    content: string;
    version: string;
    tasks: TodoTask[];
  };

  type AgentChange =
    | { operation: 'add'; title: string; afterTaskId?: string | null }
    | { operation: 'update'; taskId: string; title?: string | null; checked?: boolean | null }
    | { operation: 'delete'; taskId: string };

  type AgentProposal = {
    id: string;
    projectId: string;
    baseVersion: string;
    reason: string;
    changes: AgentChange[];
    status: 'pending' | 'accepted' | 'rejected';
    createdAt: number;
  };

  type Project = {
    id: string;
    name: string;
    root: string;
  };

  type ProjectRegistry = {
    activeProjectId: string | null;
    projects: Project[];
  };

  type View = 'tasks' | 'task-details' | 'settings' | 'projects';
  type ResizeDirection = 'NorthWest' | 'NorthEast' | 'SouthWest' | 'SouthEast';
  // A notice is rendered with the current dictionary, so it follows a language change.
  type Notice = (m: Messages) => string;

  let markdown = '';
  let newTask = '';
  let fileName = 'TODO.md';
  let language: Language = loadLanguage();
  let notice: Notice = m => m.noProjectOpen;
  let dirty = false;
  // Set when an autosave was refused because TODO.md changed on disk: autosave stops
  // and the local text is kept until the user reloads or keeps their version.
  let saveConflict: { disk: TodoSnapshot } | null = null;
  let filePath: string | null = null;
  let documentVersion = '';
  let isDesktop = false;
  let isMaximized = false;
  let isAlwaysOnTop = true;
  let fileInput: HTMLInputElement;
  let newTaskInput: HTMLInputElement;
  let proposalPoller: number | null = null;
  let unlistenResize: (() => void) | null = null;
  let unlistenDrag: (() => void) | null = null;
  let saveTimer: number | null = null;
  let titlebarDrag: HTMLDivElement;
  let view: View = 'tasks';
  let menuOpen = false;
  let projectMenuOpen = false;
  let selectedTaskId: string | null = null;
  let selectedNoteKey: string | null = null;
  let detailNote = '';
  // v2: old "task-<line>" notes do not say which file they belong to; they stay under the old key.
  const taskNotesStorageKey = 'taskhud.task-notes.v2';
  let taskNotes: Record<string, string> = {};
  let projects: Project[] = [];
  let activeProjectId: string | null = null;
  let projectNameInput = '';
  let projectRootInput = '';
  let proposals: AgentProposal[] = [];
  let proposalCollapsed = false;
  let proposalAnimationId = '';
  let proposalAnchor: HTMLElement;
  $: t = dictionaries[language];
  $: noticeText = notice(t);
  $: if (typeof document !== 'undefined') document.documentElement.lang = language;
  $: tasks = parseTodo(markdown);
  $: activeProject = projects.find(project => project.id === activeProjectId) ?? null;
  $: selectedTask = tasks.find(task => task.id === selectedTaskId) ?? null;
  $: projectName = activeProject?.name ?? (filePath ? fileName.replace(/\.[^/.]+$/, '') : t.noProject);
  $: activeProposal = isDesktop ? proposals[0] ?? null : null;
  $: proposalIsStale = Boolean(isDesktop && activeProposal && activeProposal.baseVersion !== documentVersion);
  $: conflictLocalTasks = saveConflict ? tasksMissingFrom(markdown, saveConflict.disk.content) : [];
  $: proposalBlocked =Boolean(isDesktop && activeProposal && (dirty || proposalIsStale));

  onMount(() => {
    isDesktop = '__TAURI_INTERNALS__' in window;
    document.documentElement.classList.toggle('desktop-runtime', isDesktop);
    notice = isDesktop ? m => m.noProjectOpen : m => m.noFileOpen;
    try {
      taskNotes = JSON.parse(localStorage.getItem(taskNotesStorageKey) ?? '{}') as Record<string, string>;
    } catch {
      taskNotes = {};
    }
    if (isDesktop) {
      void setupDesktopWindow();
      void loadProjects();
    }
  });

  onDestroy(() => {
    if (proposalPoller !== null) window.clearInterval(proposalPoller);
    unlistenResize?.();
    unlistenDrag?.();
    if (saveTimer !== null) window.clearTimeout(saveTimer);
    document.documentElement.classList.remove('desktop-runtime');
  });

  async function setupDesktopWindow() {
    const currentWindow = getCurrentWindow();
    const handleDrag = (event: MouseEvent) => {
      if (event.button !== 0) return;
      event.preventDefault();
      void currentWindow.startDragging().catch(error => {
        notice = errorNotice(error);
      });
    };
    titlebarDrag?.addEventListener('mousedown', handleDrag);
    unlistenDrag = () => titlebarDrag?.removeEventListener('mousedown', handleDrag);
    await syncMaximized(currentWindow);
    await syncAlwaysOnTop(currentWindow);
    unlistenResize = await currentWindow.onResized(() => void syncMaximized(currentWindow));
    proposalPoller = window.setInterval(() => {
      if (filePath) void refreshProposals();
    }, 2000);
  }

  function beginResize(direction: ResizeDirection, event: MouseEvent) {
    if (!isDesktop || isMaximized || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    void getCurrentWindow().startResizeDragging(direction).catch(error => {
      notice = errorNotice(error);
    });
  }

  async function syncMaximized(currentWindow = getCurrentWindow()) {
    try {
      isMaximized = await currentWindow.isMaximized();
    } catch {
      isMaximized = false;
    }
  }

  async function syncAlwaysOnTop(currentWindow = getCurrentWindow()) {
    try {
      isAlwaysOnTop = await currentWindow.isAlwaysOnTop();
    } catch {
      isAlwaysOnTop = true;
    }
  }

  function handleToggle(taskId: string) {
    markdown = toggleTask(markdown, taskId);
    markDirty(m => m.changeSaved);
  }

  function handleAdd() {
    if (!newTask.trim()) return;
    if (isDesktop && !filePath) {
      notice = m => m.noProjectOpen;
      return;
    }
    markdown = appendTask(markdown, newTask);
    newTask = '';
    markDirty(m => m.taskAdded);
    newTaskInput?.focus();
  }

  // A note is tied to the file (path on desktop, name on the web) and to the task title, never to its line number.
  function taskNoteKey(title: string) {
    return JSON.stringify([filePath ?? `web:${fileName}`, title]);
  }

  function openTaskDetails(taskId: string) {
    const task = tasks.find(candidate => candidate.id === taskId);
    if (!task) return;
    selectedTaskId = taskId;
    selectedNoteKey = taskNoteKey(task.title);
    detailNote = taskNotes[selectedNoteKey] ?? '';
    view = 'task-details';
    menuOpen = false;
  }

  function closeTaskDetails() {
    selectedTaskId = null;
    selectedNoteKey = null;
    detailNote = '';
    view = 'tasks';
  }

  function persistDetailNote() {
    if (!selectedNoteKey) return;
    taskNotes = { ...taskNotes, [selectedNoteKey]: detailNote };
    localStorage.setItem(taskNotesStorageKey, JSON.stringify(taskNotes));
  }

  function navigate(nextView: View) {
    view = nextView;
    menuOpen = false;
    projectMenuOpen = false;
  }

  async function loadProjects() {
    if (!isDesktop) return;
    try {
      const registry = await invoke<ProjectRegistry>('list_projects');
      projects = registry.projects;
      activeProjectId = registry.activeProjectId;
      const active = registry.projects.find(project => project.id === registry.activeProjectId);
      if (active) await openProject(active, false);
    } catch (error) {
      notice = errorNotice(error);
    }
  }

  async function openProject(project: Project, showNotice = true) {
    try {
      // The backend resolves the TODO.md path; joining it here would break on macOS and Linux.
      const snapshot = await invoke<TodoSnapshot>('open_project', { projectId: project.id });
      activeProjectId = project.id;
      applySnapshot(snapshot);
      await refreshProposals();
      projectMenuOpen = false;
      view = 'tasks';
      if (showNotice) notice = m => m.projectOpened(project.name);
    } catch (error) {
      notice = errorNotice(error);
    }
  }

  function beginCreateProject() {
    projectNameInput = '';
    projectRootInput = '';
    projectMenuOpen = false;
    navigate('projects');
  }

  async function chooseProjectRoot() {
    if (!isDesktop) return;
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === 'string') projectRootInput = selected;
  }

  async function createProject() {
    if (!isDesktop || !projectNameInput.trim()) return;
    try {
      const project = await invoke<Project>('create_project', {
        name: projectNameInput.trim(),
        root: projectRootInput,
      });
      projects = [...projects, project];
      activeProjectId = project.id;
      await openProject(project);
    } catch (error) {
      notice = errorNotice(error);
    }
  }

  function markDirty(message: Notice) {
    dirty = true;
    if (saveConflict) {
      notice = m => m.saveConflictNotice;
      return;
    }
    notice = isDesktop && filePath ? message : persistenceNotice();
    if (isDesktop && filePath) scheduleSave();
  }

  function persistenceNotice(): Notice {
    return isDesktop ? m => m.noProjectOpen : m => m.webPreview;
  }

  function scheduleSave() {
    if (saveTimer !== null) window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(() => {
      saveTimer = null;
      void saveFile();
    }, 450);
  }

  async function acceptActiveProposal() {
    if (!activeProposal) return;

    if (!isDesktop || !filePath) {
      notice = m => m.actionUnavailable;
      return;
    }
    if (proposalBlocked) {
      notice = dirty ? m => m.saving : m => m.proposalOutdated;
      return;
    }

    try {
      const snapshot = await invoke<TodoSnapshot>('accept_proposal', {
        path: filePath,
        proposalId: activeProposal.id,
      });
      applySnapshot(snapshot);
      await refreshProposals();
      notice = m => m.proposalAccepted;
    } catch (error) {
      notice = errorNotice(error);
      await refreshProposals();
    }
  }

  function toggleActiveProposal() {
    proposalCollapsed = !proposalCollapsed;
    if (!proposalCollapsed) {
      requestAnimationFrame(() => proposalAnchor?.scrollIntoView({ behavior: 'smooth', block: 'center' }));
    }
    notice = proposalCollapsed ? m => m.proposalHidden : m => m.proposalShown;
  }

  function proposalTaskTitle(change: AgentChange, m: Messages) {
    if (change.operation === 'add') return change.title;
    const task = tasks.find(candidate => candidate.id === change.taskId);
    if (change.operation === 'delete') return m.deletePrefix(task?.title ?? change.taskId);
    return change.title ?? task?.title ?? m.updatePrefix(change.taskId);
  }

  function proposalTaskLabel(change: AgentChange, m: Messages) {
    const kind = proposalTaskKind(change);
    return kind === 'add' ? m.kindAdd : kind === 'delete' ? m.kindDelete : kind === 'check' ? m.kindCheck : m.kindUpdate;
  }

  function proposalTaskKind(change: AgentChange) {
    if (change.operation === 'add') return 'add';
    if (change.operation === 'delete') return 'delete';
    return change.checked !== null && change.checked !== undefined && change.title == null ? 'check' : 'update';
  }

  function proposalTaskSymbol(change: AgentChange) {
    const kind = proposalTaskKind(change);
    return kind === 'add' ? '+' : kind === 'delete' ? '−' : kind === 'check' ? '✓' : '✎';
  }

  async function rejectActiveProposal() {
    if (!activeProposal) return;

    if (!isDesktop || !filePath) {
      notice = m => m.actionUnavailable;
      return;
    }

    try {
      await invoke<AgentProposal[]>('reject_proposal', {
        path: filePath,
        proposalId: activeProposal.id,
      });
      await refreshProposals();
      notice = m => m.proposalRejected;
    } catch (error) {
      notice = errorNotice(error);
      await refreshProposals();
    }
  }

  async function refreshProposals() {
    if (!isDesktop || !filePath) {
      proposals = [];
      return;
    }

    try {
      const previousProposalId = proposals[0]?.id ?? null;
      const nextProposals = await invoke<AgentProposal[]>('list_pending_proposals', { path: filePath });
      proposals = nextProposals;
      const nextProposalId = nextProposals[0]?.id ?? null;
      if (nextProposalId && nextProposalId !== previousProposalId) {
        proposalCollapsed = false;
        proposalAnimationId = nextProposalId;
        requestAnimationFrame(() => proposalAnchor?.scrollIntoView({ behavior: 'smooth', block: 'center' }));
        window.setTimeout(() => {
          if (proposalAnimationId === nextProposalId) proposalAnimationId = '';
        }, 1400);
      }
    } catch (error) {
      if (!activeProposal) notice = errorNotice(error);
    }
  }

  function applySnapshot(snapshot: TodoSnapshot) {
    markdown = snapshot.content;
    filePath = snapshot.path;
    documentVersion = snapshot.version;
    fileName = fileNameFromPath(snapshot.path);
    dirty = false;
    saveConflict = null;
  }

  function openFilePicker() {
    if (isDesktop) {
      projectMenuOpen = !projectMenuOpen;
      return;
    }
    fileInput?.click();
  }

  async function openDesktopFile() {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'TODO.md', extensions: ['md'] }],
      });
      const selectedPath = typeof selected === 'string' ? selected : selected?.[0];
      if (!selectedPath) {
        notice = m => m.noFileSelected;
        return;
      }

      const snapshot = await invoke<TodoSnapshot>('read_todo', { path: selectedPath });
      applySnapshot(snapshot);
      await refreshProposals();
      notice = m => m.fileOpened;
    } catch (error) {
      notice = errorNotice(error);
    }
  }

  async function saveFile() {
    if (!dirty || saveConflict) return;
    if (!isDesktop || !filePath) {
      notice = persistenceNotice();
      return;
    }
    await writeLocalVersion(documentVersion);
  }

  // Writes the local text if TODO.md is still at `expectedVersion`; on a version conflict,
  // pauses autosave and keeps the local text (see saveConflict).
  async function writeLocalVersion(expectedVersion: string) {
    if (!filePath) return;
    const path = filePath;
    const contentToSave = markdown;
    try {
      const snapshot = await invoke<TodoSnapshot>('save_todo', {
        path,
        expectedVersion,
        content: contentToSave,
      });
      if (filePath !== path) return;
      if (markdown === contentToSave) {
        applySnapshot(snapshot);
        await refreshProposals();
        notice = m => m.saved;
      } else {
        documentVersion = snapshot.version;
        dirty = true;
        saveConflict = null;
        notice = m => m.savingNext;
        scheduleSave();
      }
    } catch (error) {
      const disk = await readDisk(path);
      if (disk && disk.version !== expectedVersion && filePath === path) {
        saveConflict = { disk };
        notice = m => m.saveConflictNotice;
      } else {
        notice = errorNotice(error);
      }
    }
  }

  async function readDisk(path: string): Promise<TodoSnapshot | null> {
    try {
      return await invoke<TodoSnapshot>('read_todo', { path });
    } catch {
      return null;
    }
  }

  async function reloadFromDisk() {
    if (!filePath) return;
    try {
      applySnapshot(await invoke<TodoSnapshot>('read_todo', { path: filePath }));
      await refreshProposals();
      notice = m => m.reloadedFromDisk;
    } catch (error) {
      notice = errorNotice(error);
    }
  }

  async function keepLocalVersion() {
    if (!filePath) return;
    // Overwrites the version read just now, never an older one the user has not seen.
    const disk = await readDisk(filePath);
    if (!disk) {
      notice = errorNotice(`cannot read ${filePath}`);
      return;
    }
    saveConflict = null;
    await writeLocalVersion(disk.version);
  }

  async function windowAction(action: 'minimize' | 'toggle-maximize' | 'close') {
    if (!isDesktop) return;
    try {
      const currentWindow = getCurrentWindow();
      if (action === 'minimize') await currentWindow.minimize();
      else if (action === 'toggle-maximize') {
        await currentWindow.toggleMaximize();
        await syncMaximized(currentWindow);
      }
      else await currentWindow.close();
    } catch (error) {
      notice = errorNotice(error);
    }
  }

  async function toggleAlwaysOnTop() {
    if (!isDesktop) return;
    try {
      const currentWindow = getCurrentWindow();
      await currentWindow.setAlwaysOnTop(!isAlwaysOnTop);
      await syncAlwaysOnTop(currentWindow);
      notice = isAlwaysOnTop ? m => m.alwaysOnTop : m => m.windowFree;
    } catch (error) {
      notice = errorNotice(error);
    }
  }

  function setLanguage(next: Language) {
    language = next;
    saveLanguage(next);
  }

  function errorNotice(error: unknown): Notice {
    const text = error instanceof Error ? error.message : String(error);
    return () => text;
  }

  function fileNameFromPath(path: string) {
    return path.split(/[\\/]/).pop() || 'TODO.md';
  }

  async function loadFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    markdown = await file.text();
    filePath = null;
    documentVersion = '';
    proposals = [];
    fileName = file.name;
    dirty = false;
    notice = m => m.fileOpened;
    input.value = '';
  }
</script>

<svelte:head>
  <meta name="description" content={t.metaDescription} />
</svelte:head>

<main class="hud-shell" class:desktop-shell={isDesktop}>
  {#if isDesktop && !isMaximized}
    <div class="resize-handle resize-handle-nw" aria-hidden="true" on:mousedown={(event) => beginResize('NorthWest', event)}></div>
    <div class="resize-handle resize-handle-ne" aria-hidden="true" on:mousedown={(event) => beginResize('NorthEast', event)}></div>
    <div class="resize-handle resize-handle-sw" aria-hidden="true" on:mousedown={(event) => beginResize('SouthWest', event)}></div>
    <div class="resize-handle resize-handle-se" aria-hidden="true" on:mousedown={(event) => beginResize('SouthEast', event)}></div>
  {/if}
  <header class="topbar">
    <button class="menu-button" type="button" aria-label={t.openMenu} aria-expanded={menuOpen} on:click={() => menuOpen = !menuOpen}>☰</button>
    <div bind:this={titlebarDrag} class="titlebar-drag">
      <div class="brand-lockup">
        <h1>{view === 'settings' ? t.titleSettings : view === 'projects' ? t.titleProjects : view === 'task-details' ? t.titleDetails : t.titleTasks}</h1>
      </div>
    </div>
    {#if isDesktop}
      <div class="window-actions" aria-label={t.windowControls}>
        <button class:active={isAlwaysOnTop} class="pin-button" type="button" aria-pressed={isAlwaysOnTop} aria-label={isAlwaysOnTop ? t.pinOn : t.pinOff} title={isAlwaysOnTop ? t.pinTitleOn : t.pinTitleOff} on:click={() => void toggleAlwaysOnTop()}>⌖</button>
        <button class="window-button" type="button" aria-label={t.minimizeWindow} title={t.minimize} on:click={() => void windowAction('minimize')}>−</button>
        <button class="window-button maximize-button" type="button" aria-label={isMaximized ? t.restoreWindow : t.maximizeWindow} title={isMaximized ? t.restore : t.maximize} on:click={() => void windowAction('toggle-maximize')}>
          <span aria-hidden="true">{isMaximized ? '❐' : '□'}</span>
        </button>
        <button class="window-button close-button" type="button" aria-label={t.closeApp} title={t.close} on:click={() => void windowAction('close')}>×</button>
      </div>
    {/if}
    <input bind:this={fileInput} class="visually-hidden" type="file" accept=".md,.markdown,text/markdown" on:change={loadFile} />
  </header>

  {#if menuOpen}
    <nav class="app-menu" aria-label={t.mainMenu}>
      <button type="button" on:click={() => navigate('settings')}>{t.menuSettings}</button>
      <button type="button" on:click={() => navigate('tasks')}>{t.menuTasks}</button>
      <button type="button" on:click={() => navigate('projects')}>{t.menuProjects}</button>
    </nav>
  {/if}

  {#if view === 'tasks'}
    <div class="project-picker-wrap">
      <button class="project-select" type="button" aria-label={t.openProjectLabel(projectName)} aria-expanded={projectMenuOpen} on:click={openFilePicker}>
        <span>{projectName}</span>
        <span class="select-chevron" aria-hidden="true"></span>
      </button>
      {#if projectMenuOpen}
        <div class="project-menu" aria-label={t.savedProjects}>
          {#each projects as project (project.id)}
            <button type="button" on:click={() => void openProject(project)}>
              <span>{project.name}</span>
              <small>{project.root}</small>
            </button>
          {/each}
          {#if !projects.length}<p>{t.noSavedProject}</p>{/if}
          <button class="menu-action" type="button" on:click={beginCreateProject}>{t.newProjectAction}</button>
          <button class="menu-action" type="button" on:click={() => void openDesktopFile()}>{t.openTodoFile}</button>
        </div>
      {/if}
    </div>

    <section class="task-section" aria-labelledby="tasks-heading">
      <div class="section-heading">
        <h2 id="tasks-heading">{t.todoHeading}</h2>
        <span class="task-count" aria-label={t.taskCount(tasks.length)}>{tasks.length}</span>
      </div>

      {#if saveConflict}
        <div class="save-conflict" role="alert">
          <p><strong>{t.saveConflictTitle}</strong> {t.saveConflictBody}</p>
          {#if conflictLocalTasks.length}
            <p class="save-conflict-label">{t.saveConflictLocalOnly}</p>
            <ul>
              {#each conflictLocalTasks as task (task.id)}
                <li><span aria-hidden="true">{task.checked ? '✓' : '○'}</span> {task.title}</li>
              {/each}
            </ul>
          {/if}
          <div class="proposal-actions">
            <button class="primary-button" type="button" on:click={() => void reloadFromDisk()}>{t.reloadFromDisk}</button>
            <button class="secondary-button" type="button" on:click={() => void keepLocalVersion()}>{t.keepMyVersion}</button>
          </div>
        </div>
      {/if}

      <div class="task-stream">
      {#if isDesktop && !filePath}
        <div class="empty-project-state">
          <p class="empty-state">{t.noProjectOpen}</p>
          <div class="empty-project-actions">
            <button class="primary-button" type="button" on:click={beginCreateProject}>{t.createProject}</button>
            <button class="secondary-button" type="button" on:click={() => void openDesktopFile()}>{t.openTodoFile}</button>
          </div>
        </div>
      {:else if tasks.length || activeProposal}
        <ul class="task-list" aria-label={t.projectTasks}>
          {#each tasks as task (task.id)}
            <li class:completed={task.checked} class="task-row">
              <label class="task-check">
                <input type="checkbox" checked={task.checked} aria-label={t.markTask(task.title, task.checked)} on:change={() => handleToggle(task.id)} />
                <span class="checkmark" aria-hidden="true"></span>
              </label>
              <button class="task-title" type="button" on:click={() => openTaskDetails(task.id)}>{task.title}</button>
            </li>
          {/each}

          {#if activeProposal}
            <li bind:this={proposalAnchor} class:proposal-pulse={proposalAnimationId === activeProposal.id} class="proposal-heading-row" aria-label={t.agentProposal}>
              <div class="proposal-heading">
                <div class="proposal-title">
                  <strong id="proposal-heading">{t.agentProposal}</strong>
                  <span class="proposal-count" aria-label={t.proposedCount(activeProposal.changes.length)}>{activeProposal.changes.length}</span>
                </div>
                <button class="proposal-toggle" type="button" aria-expanded={!proposalCollapsed} on:click={toggleActiveProposal}>{proposalCollapsed ? t.show : t.hide}</button>
              </div>
              {#if !proposalCollapsed}
                <p class="proposal-reason">{activeProposal.reason}</p>
              {/if}
            </li>
            {#if !proposalCollapsed}
              {#each activeProposal.changes as change}
                <li class={`task-row proposal-task-row proposal-${proposalTaskKind(change)}`} aria-label={t.proposalItem(proposalTaskLabel(change, t), proposalTaskTitle(change, t))}>
                  <span class="task-check proposal-task-check" aria-hidden="true"><span class="checkmark proposal-checkmark">{proposalTaskSymbol(change)}</span></span>
                  <span class="task-title proposal-task-title">{proposalTaskTitle(change, t)}</span>
                </li>
              {/each}
              {#if proposalIsStale || dirty}
                <li class="proposal-meta-row">
                  {#if proposalIsStale}
                    <p class="proposal-warning">{t.proposalStaleWarning}</p>
                  {:else}
                    <p class="proposal-warning">{t.autosaveInProgress}</p>
                  {/if}
                </li>
              {/if}
              <li class="proposal-actions-row">
                <div class="proposal-actions">
                  <button class="primary-button" type="button" disabled={proposalBlocked} on:click={() => void acceptActiveProposal()}>{t.accept}</button>
                  <button class="secondary-button" type="button" on:click={() => void rejectActiveProposal()}>{t.reject}</button>
                </div>
              </li>
            {/if}
          {/if}
        </ul>
      {:else}
          <p class="empty-state">{t.noTasks}</p>
      {/if}
      </div>
      {#if !isDesktop || filePath}
        <form class="add-task" on:submit|preventDefault={handleAdd}>
          <label class="visually-hidden" for="new-task">{t.newTaskLabel}</label>
          <span class="add-mark" aria-hidden="true">+</span>
          <input bind:this={newTaskInput} id="new-task" bind:value={newTask} type="text" placeholder={t.addTaskPlaceholder} autocomplete="off" />
          <button class="add-button" type="submit">{t.add}</button>
        </form>
      {/if}
    </section>

  {:else if view === 'task-details'}
    <section class="detail-page" aria-labelledby="detail-heading">
      <button class="back-button" type="button" on:click={closeTaskDetails}>{t.backToTasks}</button>
      {#if selectedTask}
        <h2 id="detail-heading">{selectedTask.title}</h2>
        <div class="detail-status">
          <span class:detail-complete={selectedTask.checked} class="status-dot" aria-hidden="true"></span>
          <span>{selectedTask.checked ? t.statusDone : t.statusTodo}</span>
          <button class="secondary-button detail-toggle" type="button" on:click={() => handleToggle(selectedTask.id)}>{selectedTask.checked ? t.reopen : t.complete}</button>
        </div>
        <p class="detail-meta">{t.taskSource} <strong>{projectName}</strong>, {t.taskLine(selectedTask.lineIndex + 1)}</p>
        <label class="detail-note-label" for="task-note">{t.notes}</label>
        <textarea id="task-note" bind:value={detailNote} on:input={persistDetailNote} placeholder={t.notePlaceholder}></textarea>
      {:else}
        <p class="empty-state">{t.taskGone}</p>
      {/if}
    </section>
  {:else if view === 'settings'}
    <section class="settings-page" aria-labelledby="settings-heading">
      <button class="back-button" type="button" on:click={() => navigate('tasks')}>{t.backToTasks}</button>
      <h2 id="settings-heading">{t.titleSettings}</h2>
      <div class="settings-card">
        <div><strong>{t.autosave}</strong><span class="setting-state">{t.enabled}</span></div>
      </div>
      <div class="settings-card">
        <div><strong>{t.data}</strong><span class="setting-state">{t.localOnly}</span></div>
      </div>
      <div class="settings-card">
        <div>
          <label for="language-select"><strong>{t.language}</strong></label>
          <select id="language-select" class="language-select" value={language} on:change={(event) => setLanguage(event.currentTarget.value as Language)}>
            {#each languages as option (option)}
              <option value={option} lang={option}>{dictionaries[option].languageName}</option>
            {/each}
          </select>
        </div>
      </div>
    </section>
  {:else}
    <section class="projects-page" aria-labelledby="projects-heading">
      <button class="back-button" type="button" on:click={() => navigate('tasks')}>{t.backToTasks}</button>
      <h2 id="projects-heading">{t.newProject}</h2>
      {#if isDesktop}
        <form class="project-form" on:submit|preventDefault={() => void createProject()}>
          <label for="project-name">{t.projectName}</label>
          <input id="project-name" bind:value={projectNameInput} placeholder={t.projectNamePlaceholder} />
          <label for="project-root">{t.externalFolder} <span class="optional-label">{t.optional}</span></label>
          <div class="folder-picker">
            <input id="project-root" value={projectRootInput} readonly placeholder={t.noFolder} />
            <button class="secondary-button" type="button" on:click={() => void chooseProjectRoot()}>{t.choose}</button>
          </div>
          <button class="primary-button" type="submit" disabled={!projectNameInput.trim()}>{t.createTheProject}</button>
        </form>
      {:else}
        <p class="empty-state">{t.desktopOnlyProjects}</p>
      {/if}
      {#if projects.length}
        <div class="registered-projects">
          <h3>{t.savedProjects}</h3>
          {#each projects as project (project.id)}
            <button type="button" class:active-project={project.id === activeProjectId} on:click={() => void openProject(project)}><strong>{project.name}</strong><small>{project.root}</small></button>
          {/each}
        </div>
      {/if}
    </section>
  {/if}

  <footer class="status-bar" role="status" aria-live="polite">
    <span class:is-dirty={dirty} class="status-dot" aria-hidden="true"></span>
    <span>{noticeText}</span>
  </footer>
</main>
