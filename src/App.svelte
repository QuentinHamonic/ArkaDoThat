<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open } from '@tauri-apps/plugin-dialog';
  import { applyProposal, appendTask, parseTodo, toggleTask, type Proposal } from './lib/todo';

  type TodoSnapshot = {
    path: string;
    content: string;
    version: string;
    tasks: Array<{ id: string; title: string; checked: boolean; lineIndex: number }>;
  };

  const demoMarkdown = `# TaskHUD\n\n## À faire\n- [ ] Examiner la proposition de l'agent\n- [ ] Vérifier le rendu à 320 px\n- [x] Écrire le contrat QA\n`;

  let markdown = demoMarkdown;
  let newTask = '';
  let fileName = 'TODO.md';
  let sourceLabel = 'Démo locale';
  let notice = 'Prêt.';
  let dirty = false;
  let filePath: string | null = null;
  let documentVersion = '';
  let isDesktop = false;
  let fileInput: HTMLInputElement;
  let newTaskInput: HTMLInputElement;
  let proposal: Proposal | null = {
    operation: 'add',
    title: 'Proposition de l’agent',
    after: 'Ajouter une vérification de persistance',
  };

  $: tasks = parseTodo(markdown);
  $: projectName = fileName === 'TODO.md' && sourceLabel === 'Démo locale'
    ? 'Projet local'
    : fileName.replace(/\.[^/.]+$/, '');

  onMount(() => {
    isDesktop = '__TAURI_INTERNALS__' in window;
  });

  function handleToggle(taskId: string) {
    markdown = toggleTask(markdown, taskId);
    dirty = true;
    notice = 'Modification locale.';
  }

  function handleAdd() {
    if (!newTask.trim()) return;
    markdown = appendTask(markdown, newTask);
    newTask = '';
    dirty = true;
    notice = 'Tâche ajoutée.';
    newTaskInput?.focus();
  }

  function acceptProposal() {
    if (!proposal) return;
    try {
      markdown = applyProposal(markdown, proposal);
      proposal = null;
      dirty = true;
      notice = 'Proposition acceptée.';
    } catch (error) {
      notice = error instanceof Error ? error.message : 'Proposition impossible à appliquer.';
    }
  }

  function rejectProposal() {
    proposal = null;
    notice = 'Proposition ignorée.';
  }

  function proposalOperationLabel(operation: Proposal['operation']) {
    return operation === 'add' ? 'Ajout' : operation === 'edit' ? 'Modification' : 'Suppression';
  }

  function openFilePicker() {
    if (isDesktop) {
      void openDesktopFile();
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
      if (typeof selected !== 'string') return;

      const snapshot = await invoke<TodoSnapshot>('read_todo', { path: selected });
      markdown = snapshot.content;
      filePath = snapshot.path;
      documentVersion = snapshot.version;
      fileName = fileNameFromPath(snapshot.path);
      sourceLabel = 'Fichier local';
      dirty = false;
      notice = 'Fichier ouvert.';
    } catch (error) {
      notice = error instanceof Error ? error.message : String(error);
    }
  }

  async function saveFile() {
    if (!dirty) return;
    if (!isDesktop || !filePath) {
      notice = 'Disponible dans TaskHUD desktop.';
      return;
    }

    try {
      const snapshot = await invoke<TodoSnapshot>('save_todo', {
        path: filePath,
        expectedVersion: documentVersion,
        content: markdown,
      });
      documentVersion = snapshot.version;
      dirty = false;
      notice = 'Enregistré.';
    } catch (error) {
      notice = error instanceof Error ? error.message : String(error);
    }
  }

  async function windowAction(action: 'minimize' | 'close') {
    if (!isDesktop) return;
    const currentWindow = getCurrentWindow();
    if (action === 'minimize') await currentWindow.minimize();
    else await currentWindow.close();
  }

  function fileNameFromPath(path: string) {
    return path.split(/[\\/]/).pop() || 'TODO.md';
  }

  async function loadFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    markdown = await file.text();
    fileName = file.name;
    sourceLabel = 'Fichier local';
    dirty = false;
    notice = 'Fichier ouvert.';
    input.value = '';
  }
</script>

<svelte:head>
  <meta name="description" content="Un overlay local et discret pour les tâches d'un projet." />
</svelte:head>

<main class="hud-shell">
  <header class="topbar">
    <div class="brand-lockup">
      <span class="list-mark" aria-hidden="true"><i></i><i></i><i></i></span>
      <h1>Tasks</h1>
    </div>
    {#if isDesktop}
      <div class="window-actions" aria-label="Contrôles de fenêtre">
        <span class="pin-indicator" title="Toujours au premier plan" aria-label="Toujours au premier plan">⌖</span>
        <button class="window-button" type="button" aria-label="Réduire la fenêtre" on:click={() => void windowAction('minimize')}>−</button>
        <button class="window-button close-button" type="button" aria-label="Fermer TaskHUD" on:click={() => void windowAction('close')}>×</button>
      </div>
    {/if}
    <input bind:this={fileInput} class="visually-hidden" type="file" accept=".md,.markdown,text/markdown" on:change={loadFile} />
  </header>

  <button class="project-select" type="button" aria-label={`Ouvrir le projet ${projectName}`} on:click={openFilePicker}>
    <span>{projectName}</span>
    <span class="select-chevron" aria-hidden="true">⌄</span>
  </button>

  <section class="task-section" aria-labelledby="tasks-heading">
    <div class="section-heading">
      <h2 id="tasks-heading">À faire</h2>
      <span class="task-count">{tasks.length}</span>
    </div>

    {#if tasks.length}
      <ul class="task-list" aria-label="Tâches du projet">
        {#each tasks as task (task.id)}
          <li class:completed={task.checked} class="task-row">
            <label class="task-label">
              <input type="checkbox" checked={task.checked} aria-label={`Marquer « ${task.title} » comme ${task.checked ? 'non terminée' : 'terminée'}`} on:change={() => handleToggle(task.id)} />
              <span class="checkmark" aria-hidden="true"></span>
              <span>{task.title}</span>
            </label>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty-state">Aucune tâche. Ajoutez le prochain geste.</p>
    {/if}

    <form class="add-task" on:submit|preventDefault={handleAdd}>
      <label class="visually-hidden" for="new-task">Nouvelle tâche</label>
      <span class="add-mark" aria-hidden="true">+</span>
      <input bind:this={newTaskInput} id="new-task" bind:value={newTask} type="text" placeholder="Ajouter une tâche" autocomplete="off" />
      <button class="add-button" type="submit">Ajouter</button>
    </form>
  </section>

  {#if proposal}
    <section class="proposal" aria-labelledby="proposal-heading">
      <div class="proposal-heading">
        <div class="proposal-title">
          <span class="proposal-branch" aria-hidden="true">⑂</span>
          <strong id="proposal-heading">Proposition de l’agent</strong>
          <span class="proposal-count">1</span>
        </div>
        <span class="proposal-kind">{proposalOperationLabel(proposal.operation)}</span>
      </div>
      <div class="proposal-change">
        {#if proposal.before}<p class="diff-remove">− {proposal.before}</p>{/if}
        {#if proposal.after}<p class="diff-add">+ {proposal.after}</p>{/if}
      </div>
      <div class="proposal-actions">
        <button class="primary-button" type="button" on:click={acceptProposal}>Accepter</button>
        <button class="secondary-button" type="button" on:click={rejectProposal}>Ignorer</button>
      </div>
    </section>
  {/if}

  <footer class="status-bar" role="status" aria-live="polite">
    <span class:is-dirty={dirty} class="status-dot" aria-hidden="true"></span>
    <span>{notice}</span>
    {#if dirty}<button class="save-link" type="button" on:click={() => void saveFile()}>Enregistrer</button>{/if}
  </footer>
</main>
