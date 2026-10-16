<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
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
  let proposalOpen = false;
  let fileInput: HTMLInputElement;
  let newTaskInput: HTMLInputElement;
  let proposal: Proposal | null = {
    operation: 'add',
    title: 'Proposition de l’agent',
    after: 'Ajouter une vérification de persistance',
  };

  $: tasks = parseTodo(markdown);
  $: completedCount = tasks.filter(task => task.checked).length;
  $: pendingCount = tasks.length - completedCount;

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
      proposalOpen = false;
      dirty = true;
      notice = 'Proposition acceptée.';
    } catch (error) {
      notice = error instanceof Error ? error.message : 'Proposition impossible à appliquer.';
    }
  }

  function rejectProposal() {
    proposal = null;
    proposalOpen = false;
    notice = 'Proposition refusée.';
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
      <span class="signal" aria-hidden="true"></span>
      <div class="project-lockup">
        <p class="brand">TaskHUD</p>
        <p class="project-name" title={sourceLabel}>{fileName}</p>
      </div>
    </div>
    <div class="topbar-actions">
      <button class="icon-button" type="button" aria-label="Ouvrir un fichier TODO.md" title="Ouvrir TODO.md" on:click={openFilePicker}>↗</button>
      <button class="icon-button" class:save-ready={dirty} type="button" aria-label="Enregistrer TODO.md" title="Enregistrer TODO.md" disabled={!dirty} on:click={() => void saveFile()}>↓</button>
    </div>
    <input bind:this={fileInput} class="visually-hidden" type="file" accept=".md,.markdown,text/markdown" on:change={loadFile} />
  </header>

  <section class="summary" aria-label="Résumé des tâches">
    <div class="summary-copy">
      <strong>{pendingCount} à faire</strong>
      <span>{completedCount}/{tasks.length} terminées</span>
    </div>
    <span class:dirty class="sync-state">{dirty ? 'À enregistrer' : 'À jour'}</span>
    <div class="progress-track" aria-hidden="true"><span style={`width: ${tasks.length ? (completedCount / tasks.length) * 100 : 0}%`}></span></div>
  </section>

  <section class="task-section" aria-labelledby="tasks-heading">
    <div class="section-heading">
      <h1 id="tasks-heading">À faire</h1>
      <span class="task-count">{tasks.length}</span>
    </div>

    {#if tasks.length}
      <ul class="task-list" aria-label="Tâches du projet">
        {#each tasks as task (task.id)}
          <li class:completed={task.checked} class="task-row">
            <label class="task-label">
              <input type="checkbox" checked={task.checked} aria-label={`Marquer « ${task.title} » comme ${task.checked ? 'non terminée' : 'terminée'}`} on:change={() => handleToggle(task.id)} />
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
      <button class="proposal-toggle" type="button" aria-expanded={proposalOpen} on:click={() => proposalOpen = !proposalOpen}>
        <span class="proposal-dot" aria-hidden="true"></span>
        <span class="proposal-summary">
          <strong id="proposal-heading">1 proposition · {proposalOperationLabel(proposal.operation)}</strong>
          <span>{proposal.title}</span>
        </span>
        <span class="proposal-chevron" aria-hidden="true">{proposalOpen ? '⌃' : '⌄'}</span>
      </button>
      {#if proposalOpen}
        <div class="proposal-details">
          <div class="diff" aria-label="Diff de la proposition">
            <p class="diff-line diff-context">{fileName}</p>
            {#if proposal.before}<p class="diff-line diff-remove">− {proposal.before}</p>{/if}
            {#if proposal.after}<p class="diff-line diff-add">+ {proposal.after}</p>{/if}
          </div>
          <p class="proposal-note">Rien ne change avant votre décision.</p>
          <div class="proposal-actions">
            <button class="secondary-button" type="button" on:click={rejectProposal}>Refuser</button>
            <button class="primary-button" type="button" on:click={acceptProposal}>Accepter</button>
          </div>
        </div>
      {/if}
    </section>
  {/if}

  <footer class="status-bar" role="status" aria-live="polite">
    <span class:is-dirty={dirty} class="status-dot" aria-hidden="true"></span>
    <span>{notice}</span>
  </footer>
</main>
