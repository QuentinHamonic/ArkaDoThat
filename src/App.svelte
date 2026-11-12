<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-dialog';
  import { appendTask, parseTodo, toggleTask } from './lib/todo';

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

  let markdown = '';
  let newTask = '';
  let fileName = 'TODO.md';
  let notice = 'Aucun projet ouvert.';
  let dirty = false;
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
  let detailNote = '';
  let taskNotes: Record<string, string> = {};
  let projects: Project[] = [];
  let activeProjectId: string | null = null;
  let projectNameInput = '';
  let projectRootInput = '';
  let proposals: AgentProposal[] = [];
  let proposalCollapsed = false;
  let proposalAnimationId = '';
  let proposalAnchor: HTMLElement;
  $: tasks = parseTodo(markdown);
  $: activeProject = projects.find(project => project.id === activeProjectId) ?? null;
  $: selectedTask = tasks.find(task => task.id === selectedTaskId) ?? null;
  $: projectName = activeProject?.name ?? (filePath ? fileName.replace(/\.[^/.]+$/, '') : 'Aucun projet');
  $: activeProposal = isDesktop ? proposals[0] ?? null : null;
  $: proposalIsStale = Boolean(isDesktop && activeProposal && activeProposal.baseVersion !== documentVersion);
  $: proposalBlocked = Boolean(isDesktop && activeProposal && (dirty || proposalIsStale));

  onMount(() => {
    isDesktop = '__TAURI_INTERNALS__' in window;
    document.documentElement.classList.toggle('desktop-runtime', isDesktop);
    notice = isDesktop ? 'Aucun projet ouvert.' : 'Aucun fichier ouvert.';
    try {
      taskNotes = JSON.parse(localStorage.getItem('taskhud.task-notes') ?? '{}') as Record<string, string>;
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
        notice = error instanceof Error ? error.message : String(error);
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
      notice = error instanceof Error ? error.message : String(error);
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
    markDirty('Modification enregistrée.');
  }

  function handleAdd() {
    if (!newTask.trim()) return;
    if (isDesktop && !filePath) {
      notice = 'Aucun projet ouvert.';
      return;
    }
    markdown = appendTask(markdown, newTask);
    newTask = '';
    markDirty('Tâche ajoutée.');
    newTaskInput?.focus();
  }

  function openTaskDetails(taskId: string) {
    selectedTaskId = taskId;
    detailNote = taskNotes[taskId] ?? '';
    view = 'task-details';
    menuOpen = false;
  }

  function closeTaskDetails() {
    selectedTaskId = null;
    detailNote = '';
    view = 'tasks';
  }

  function persistDetailNote() {
    if (!selectedTaskId) return;
    taskNotes = { ...taskNotes, [selectedTaskId]: detailNote };
    localStorage.setItem('taskhud.task-notes', JSON.stringify(taskNotes));
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
      notice = error instanceof Error ? error.message : String(error);
    }
  }

  async function openProject(project: Project, showNotice = true) {
    try {
      const snapshot = await invoke<TodoSnapshot>('read_todo', { path: `${project.root}\\TODO.md` });
      activeProjectId = project.id;
      applySnapshot(snapshot);
      await invoke('set_active_project', { projectId: project.id });
      await refreshProposals();
      projectMenuOpen = false;
      view = 'tasks';
      if (showNotice) notice = `Projet « ${project.name} » ouvert.`;
    } catch (error) {
      notice = error instanceof Error ? error.message : String(error);
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
      notice = error instanceof Error ? error.message : String(error);
    }
  }

  function markDirty(message: string) {
    dirty = true;
    notice = isDesktop && filePath ? message : persistenceNotice();
    if (isDesktop && filePath) scheduleSave();
  }

  function persistenceNotice() {
    return isDesktop ? 'Aucun projet ouvert.' : 'Aperçu web.';
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
      notice = 'Action indisponible ici.';
      return;
    }
    if (proposalBlocked) {
      notice = dirty
        ? 'Enregistrement en cours.'
        : 'Cette proposition est basée sur une version ancienne. Rechargez le fichier.';
      return;
    }

    try {
      const snapshot = await invoke<TodoSnapshot>('accept_proposal', {
        path: filePath,
        proposalId: activeProposal.id,
      });
      applySnapshot(snapshot);
      await refreshProposals();
      notice = 'Proposition acceptée et enregistrée.';
    } catch (error) {
      notice = error instanceof Error ? error.message : String(error);
      await refreshProposals();
    }
  }

  function toggleActiveProposal() {
    proposalCollapsed = !proposalCollapsed;
    if (!proposalCollapsed) {
      requestAnimationFrame(() => proposalAnchor?.scrollIntoView({ behavior: 'smooth', block: 'center' }));
    }
    notice = proposalCollapsed ? 'Proposition masquée.' : 'Proposition affichée.';
  }

  function proposalTaskTitle(change: AgentChange) {
    if (change.operation === 'add') return change.title;
    const task = tasks.find(candidate => candidate.id === change.taskId);
    if (change.operation === 'delete') return `Supprimer : ${task?.title ?? change.taskId}`;
    return change.title ?? task?.title ?? `Modifier : ${change.taskId}`;
  }

  function proposalTaskLabel(change: AgentChange) {
    if (change.operation === 'add') return 'Nouvelle tâche';
    if (change.operation === 'delete') return 'Suppression';
    return change.checked !== null && change.checked !== undefined && change.title == null
      ? 'Tâche à cocher'
      : 'Modification';
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
      notice = 'Action indisponible ici.';
      return;
    }

    try {
      await invoke<AgentProposal[]>('reject_proposal', {
        path: filePath,
        proposalId: activeProposal.id,
      });
      await refreshProposals();
      notice = 'Proposition refusée.';
    } catch (error) {
      notice = error instanceof Error ? error.message : String(error);
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
      if (!activeProposal) notice = error instanceof Error ? error.message : String(error);
    }
  }

  function applySnapshot(snapshot: TodoSnapshot) {
    markdown = snapshot.content;
    filePath = snapshot.path;
    documentVersion = snapshot.version;
    fileName = fileNameFromPath(snapshot.path);
    dirty = false;
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
        notice = 'Aucun fichier sélectionné.';
        return;
      }

      const snapshot = await invoke<TodoSnapshot>('read_todo', { path: selectedPath });
      applySnapshot(snapshot);
      await refreshProposals();
      notice = 'Fichier ouvert.';
    } catch (error) {
      notice = error instanceof Error ? error.message : String(error);
    }
  }

  async function saveFile() {
    if (!dirty) return;
    if (!isDesktop || !filePath) {
      notice = persistenceNotice();
      return;
    }

    const contentToSave = markdown;
    const versionToSave = documentVersion;
    try {
      const snapshot = await invoke<TodoSnapshot>('save_todo', {
        path: filePath,
        expectedVersion: versionToSave,
        content: contentToSave,
      });
      if (markdown === contentToSave && documentVersion === versionToSave) {
        applySnapshot(snapshot);
        await refreshProposals();
        notice = 'Enregistré.';
      } else {
        documentVersion = snapshot.version;
        dirty = true;
        notice = 'Modification suivante en cours d’enregistrement…';
        scheduleSave();
      }
    } catch (error) {
      notice = error instanceof Error ? error.message : String(error);
    }
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
      notice = error instanceof Error ? error.message : String(error);
    }
  }

  async function toggleAlwaysOnTop() {
    if (!isDesktop) return;
    try {
      const currentWindow = getCurrentWindow();
      await currentWindow.setAlwaysOnTop(!isAlwaysOnTop);
      await syncAlwaysOnTop(currentWindow);
      notice = isAlwaysOnTop ? 'Fenêtre toujours au premier plan.' : 'Fenêtre libre.';
    } catch (error) {
      notice = error instanceof Error ? error.message : String(error);
    }
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
    notice = 'Fichier ouvert.';
    input.value = '';
  }
</script>

<svelte:head>
  <meta name="description" content="Un overlay local et discret pour les tâches d'un projet." />
</svelte:head>

<main class="hud-shell" class:desktop-shell={isDesktop}>
  {#if isDesktop && !isMaximized}
    <div class="resize-handle resize-handle-nw" aria-hidden="true" on:mousedown={(event) => beginResize('NorthWest', event)}></div>
    <div class="resize-handle resize-handle-ne" aria-hidden="true" on:mousedown={(event) => beginResize('NorthEast', event)}></div>
    <div class="resize-handle resize-handle-sw" aria-hidden="true" on:mousedown={(event) => beginResize('SouthWest', event)}></div>
    <div class="resize-handle resize-handle-se" aria-hidden="true" on:mousedown={(event) => beginResize('SouthEast', event)}></div>
  {/if}
  <header class="topbar">
    <button class="menu-button" type="button" aria-label="Ouvrir le menu" aria-expanded={menuOpen} on:click={() => menuOpen = !menuOpen}>☰</button>
    <div bind:this={titlebarDrag} class="titlebar-drag">
      <div class="brand-lockup">
        <h1>{view === 'settings' ? 'Paramètres' : view === 'projects' ? 'Projets' : view === 'task-details' ? 'Détails' : 'Tasks'}</h1>
      </div>
    </div>
    {#if isDesktop}
      <div class="window-actions" aria-label="Contrôles de fenêtre">
        <button class:active={isAlwaysOnTop} class="pin-button" type="button" aria-pressed={isAlwaysOnTop} aria-label={isAlwaysOnTop ? 'Désactiver le premier plan' : 'Activer le premier plan'} title={isAlwaysOnTop ? 'Toujours au premier plan : activé' : 'Toujours au premier plan : désactivé'} on:click={() => void toggleAlwaysOnTop()}>⌖</button>
        <button class="window-button" type="button" aria-label="Réduire la fenêtre" title="Réduire" on:click={() => void windowAction('minimize')}>−</button>
        <button class="window-button maximize-button" type="button" aria-label={isMaximized ? 'Restaurer la fenêtre' : 'Agrandir la fenêtre'} title={isMaximized ? 'Restaurer' : 'Agrandir'} on:click={() => void windowAction('toggle-maximize')}>
          <span aria-hidden="true">{isMaximized ? '❐' : '□'}</span>
        </button>
        <button class="window-button close-button" type="button" aria-label="Fermer TaskHUD" title="Fermer" on:click={() => void windowAction('close')}>×</button>
      </div>
    {/if}
    <input bind:this={fileInput} class="visually-hidden" type="file" accept=".md,.markdown,text/markdown" on:change={loadFile} />
  </header>

  {#if menuOpen}
    <nav class="app-menu" aria-label="Menu principal">
      <button type="button" on:click={() => navigate('settings')}>Paramètres</button>
      <button type="button" on:click={() => navigate('tasks')}>Tâches</button>
      <button type="button" on:click={() => navigate('projects')}>Projets</button>
    </nav>
  {/if}

  {#if view === 'tasks'}
    <div class="project-picker-wrap">
      <button class="project-select" type="button" aria-label={`Ouvrir le projet ${projectName}`} aria-expanded={projectMenuOpen} on:click={openFilePicker}>
        <span>{projectName}</span>
        <span class="select-chevron" aria-hidden="true"></span>
      </button>
      {#if projectMenuOpen}
        <div class="project-menu" aria-label="Projets enregistrés">
          {#each projects as project (project.id)}
            <button type="button" on:click={() => void openProject(project)}>
              <span>{project.name}</span>
              <small>{project.root}</small>
            </button>
          {/each}
          {#if !projects.length}<p>Aucun projet enregistré.</p>{/if}
          <button class="menu-action" type="button" on:click={beginCreateProject}>+ Nouveau projet</button>
          <button class="menu-action" type="button" on:click={() => void openDesktopFile()}>Ouvrir un TODO.md</button>
        </div>
      {/if}
    </div>

    <section class="task-section" aria-labelledby="tasks-heading">
      <div class="section-heading">
        <h2 id="tasks-heading">À faire</h2>
        <span class="task-count" aria-label={`${tasks.length} tâche${tasks.length > 1 ? 's' : ''}`}>{tasks.length}</span>
      </div>

      <div class="task-stream">
      {#if isDesktop && !filePath}
        <div class="empty-project-state">
          <p class="empty-state">Aucun projet ouvert.</p>
          <div class="empty-project-actions">
            <button class="primary-button" type="button" on:click={beginCreateProject}>Créer un projet</button>
            <button class="secondary-button" type="button" on:click={() => void openDesktopFile()}>Ouvrir un TODO.md</button>
          </div>
        </div>
      {:else if tasks.length || activeProposal}
        <ul class="task-list" aria-label="Tâches du projet">
          {#each tasks as task (task.id)}
            <li class:completed={task.checked} class="task-row">
              <label class="task-check">
                <input type="checkbox" checked={task.checked} aria-label={`Marquer « ${task.title} » comme ${task.checked ? 'non terminée' : 'terminée'}`} on:change={() => handleToggle(task.id)} />
                <span class="checkmark" aria-hidden="true"></span>
              </label>
              <button class="task-title" type="button" on:click={() => openTaskDetails(task.id)}>{task.title}</button>
            </li>
          {/each}

          {#if activeProposal}
            <li bind:this={proposalAnchor} class:proposal-pulse={proposalAnimationId === activeProposal.id} class="proposal-heading-row" aria-label="Proposition de l’agent">
              <div class="proposal-heading">
                <div class="proposal-title">
                  <strong id="proposal-heading">Proposition de l’agent</strong>
                  <span class="proposal-count" aria-label={`${activeProposal.changes.length} tâche${activeProposal.changes.length > 1 ? 's' : ''} proposée${activeProposal.changes.length > 1 ? 's' : ''}`}>{activeProposal.changes.length}</span>
                </div>
                <button class="proposal-toggle" type="button" aria-expanded={!proposalCollapsed} on:click={toggleActiveProposal}>{proposalCollapsed ? 'Afficher' : 'Masquer'}</button>
              </div>
              {#if !proposalCollapsed}
                <p class="proposal-reason">{activeProposal.reason}</p>
              {/if}
            </li>
            {#if !proposalCollapsed}
              {#each activeProposal.changes as change}
                <li class={`task-row proposal-task-row proposal-${proposalTaskKind(change)}`} aria-label={`Proposition : ${proposalTaskLabel(change)} — ${proposalTaskTitle(change)}`}>
                  <span class="task-check proposal-task-check" aria-hidden="true"><span class="checkmark proposal-checkmark">{proposalTaskSymbol(change)}</span></span>
                  <span class="task-title proposal-task-title">{proposalTaskTitle(change)}</span>
                </li>
              {/each}
              {#if proposalIsStale || dirty}
                <li class="proposal-meta-row">
                  {#if proposalIsStale}
                    <p class="proposal-warning">Cette proposition ne correspond plus à la version affichée.</p>
                  {:else}
                    <p class="proposal-warning">Enregistrement automatique en cours.</p>
                  {/if}
                </li>
              {/if}
              <li class="proposal-actions-row">
                <div class="proposal-actions">
                  <button class="primary-button" type="button" disabled={proposalBlocked} on:click={() => void acceptActiveProposal()}>Accepter</button>
                  <button class="secondary-button" type="button" on:click={() => void rejectActiveProposal()}>Refuser</button>
                </div>
              </li>
            {/if}
          {/if}
        </ul>
      {:else}
          <p class="empty-state">Aucune tâche. Ajoutez le prochain geste.</p>
      {/if}
      </div>
      {#if !isDesktop || filePath}
        <form class="add-task" on:submit|preventDefault={handleAdd}>
          <label class="visually-hidden" for="new-task">Nouvelle tâche</label>
          <span class="add-mark" aria-hidden="true">+</span>
          <input bind:this={newTaskInput} id="new-task" bind:value={newTask} type="text" placeholder="Ajouter une tâche" autocomplete="off" />
          <button class="add-button" type="submit">Ajouter</button>
        </form>
      {/if}
    </section>

  {:else if view === 'task-details'}
    <section class="detail-page" aria-labelledby="detail-heading">
      <button class="back-button" type="button" on:click={closeTaskDetails}>← Retour aux tâches</button>
      {#if selectedTask}
        <h2 id="detail-heading">{selectedTask.title}</h2>
        <div class="detail-status">
          <span class:detail-complete={selectedTask.checked} class="status-dot" aria-hidden="true"></span>
          <span>{selectedTask.checked ? 'Terminée' : 'À faire'}</span>
          <button class="secondary-button detail-toggle" type="button" on:click={() => handleToggle(selectedTask.id)}>{selectedTask.checked ? 'Rouvrir' : 'Terminer'}</button>
        </div>
        <p class="detail-meta">Cette tâche vient de <strong>{projectName}</strong>, ligne {selectedTask.lineIndex + 1}.</p>
        <label class="detail-note-label" for="task-note">Notes</label>
        <textarea id="task-note" bind:value={detailNote} on:input={persistDetailNote} placeholder="Ajouter une note personnelle…"></textarea>
      {:else}
        <p class="empty-state">Cette tâche n’existe plus dans le document courant.</p>
      {/if}
    </section>
  {:else if view === 'settings'}
    <section class="settings-page" aria-labelledby="settings-heading">
      <button class="back-button" type="button" on:click={() => navigate('tasks')}>← Retour aux tâches</button>
      <h2 id="settings-heading">Paramètres</h2>
      <div class="settings-card">
        <div><strong>Enregistrement automatique</strong><span class="setting-state">Activé</span></div>
      </div>
      <div class="settings-card">
        <div><strong>Données</strong><span class="setting-state">Local uniquement</span></div>
      </div>
    </section>
  {:else}
    <section class="projects-page" aria-labelledby="projects-heading">
      <button class="back-button" type="button" on:click={() => navigate('tasks')}>← Retour aux tâches</button>
      <h2 id="projects-heading">Nouveau projet</h2>
      {#if isDesktop}
        <form class="project-form" on:submit|preventDefault={() => void createProject()}>
          <label for="project-name">Nom du projet</label>
          <input id="project-name" bind:value={projectNameInput} placeholder="Ex. Révisions de maths" />
          <label for="project-root">Dossier externe <span class="optional-label">(facultatif)</span></label>
          <div class="folder-picker">
            <input id="project-root" value={projectRootInput} readonly placeholder="Aucun dossier" />
            <button class="secondary-button" type="button" on:click={() => void chooseProjectRoot()}>Choisir</button>
          </div>
          <button class="primary-button" type="submit" disabled={!projectNameInput.trim()}>Créer le projet</button>
        </form>
      {:else}
        <p class="empty-state">La création de projets est disponible dans l’application desktop.</p>
      {/if}
      {#if projects.length}
        <div class="registered-projects">
          <h3>Projets enregistrés</h3>
          {#each projects as project (project.id)}
            <button type="button" class:active-project={project.id === activeProjectId} on:click={() => void openProject(project)}><strong>{project.name}</strong><small>{project.root}</small></button>
          {/each}
        </div>
      {/if}
    </section>
  {/if}

  <footer class="status-bar" role="status" aria-live="polite">
    <span class:is-dirty={dirty} class="status-dot" aria-hidden="true"></span>
    <span>{notice}</span>
  </footer>
</main>
