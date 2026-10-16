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
  let notice = 'Aucune modification en attente.';
  let dirty = false;
  let filePath: string | null = null;
  let documentVersion = '';
  let isDesktop = false;
  let fileInput: HTMLInputElement;
  let proposal: Proposal | null = {
    operation: 'add',
    title: 'Proposition d’agent',
    after: 'Ajouter une vérification de persistance',
  };

  $: tasks = parseTodo(markdown);
  $: completedCount = tasks.filter(task => task.checked).length;

  onMount(() => {
    isDesktop = '__TAURI_INTERNALS__' in window;
  });

  function handleToggle(taskId: string) {
    markdown = toggleTask(markdown, taskId);
    dirty = true;
    notice = 'État modifié localement.';
  }

  function handleAdd() {
    if (!newTask.trim()) return;
    markdown = appendTask(markdown, newTask);
    newTask = '';
    dirty = true;
    notice = 'Tâche ajoutée localement.';
  }

  function acceptProposal() {
    if (!proposal) return;
    try {
      markdown = applyProposal(markdown, proposal);
      proposal = null;
      dirty = true;
      notice = 'Proposition acceptée. Le diff est appliqué localement.';
    } catch (error) {
      notice = error instanceof Error ? error.message : 'Proposition impossible à appliquer.';
    }
  }

  function rejectProposal() {
    proposal = null;
    notice = 'Proposition refusée. La liste reste inchangée.';
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
      sourceLabel = 'Fichier chargé';
      dirty = false;
      notice = 'Fichier chargé depuis le disque.';
    } catch (error) {
      notice = error instanceof Error ? error.message : String(error);
    }
  }

  async function saveFile() {
    if (!dirty) return;
    if (!isDesktop || !filePath) {
      notice = 'La sauvegarde disque est disponible dans l’application TaskHUD desktop.';
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
      notice = 'TODO.md enregistré.';
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
    sourceLabel = 'Fichier chargé';
    dirty = false;
    notice = 'Fichier chargé en mémoire. La sauvegarde Tauri viendra dans la tranche desktop.';
    input.value = '';
  }
</script>

<svelte:head>
  <meta name="description" content="Une liste TODO locale, compacte et lisible pour les projets avec agents IA." />
</svelte:head>

<main class="hud-shell">
  <header class="topbar">
    <div class="brand-lockup">
      <span class="signal" aria-hidden="true"></span>
      <div>
        <p class="brand">TaskHUD</p>
        <p class="subtitle">un regard sur le prochain geste</p>
      </div>
    </div>
    <div class="topbar-actions">
      <button class="icon-button" type="button" aria-label="Ouvrir un fichier TODO.md" title="Ouvrir TODO.md" on:click={openFilePicker}>↗</button>
      <button class="icon-button" class:save-ready={dirty} type="button" aria-label="Enregistrer TODO.md" title="Enregistrer TODO.md" disabled={!dirty} on:click={() => void saveFile()}>↓</button>
    </div>
    <input bind:this={fileInput} class="visually-hidden" type="file" accept=".md,.markdown,text/markdown" on:change={loadFile} />
  </header>

  <section class="source-strip" aria-label="Source active">
    <div>
      <span class="eyebrow">Projet actif</span>
      <strong>{fileName}</strong>
    </div>
    <span class:dirty class="source-state">{dirty ? 'modifié localement' : sourceLabel}</span>
  </section>

  <section class="progress-block" aria-label="Progression des tâches">
    <div class="progress-copy">
      <span><strong>{completedCount}</strong> / {tasks.length} terminées</span>
      <span class="quiet">source lisible · Markdown</span>
    </div>
    <div class="progress-track" aria-hidden="true"><span style={`width: ${tasks.length ? (completedCount / tasks.length) * 100 : 0}%`}></span></div>
  </section>

  <section class="task-section" aria-labelledby="tasks-heading">
    <div class="section-heading">
      <div>
        <span class="eyebrow">Maintenant</span>
        <h1 id="tasks-heading">À faire</h1>
      </div>
      <span class="task-count">{tasks.length} tâches</span>
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
      <p class="empty-state">Aucune tâche dans ce fichier. Ajoutez le prochain geste ci-dessous.</p>
    {/if}

    <form class="add-task" on:submit|preventDefault={handleAdd}>
      <label class="visually-hidden" for="new-task">Nouvelle tâche</label>
      <input id="new-task" bind:value={newTask} type="text" placeholder="Ajouter le prochain geste…" autocomplete="off" />
      <button class="add-button" type="submit">Ajouter</button>
    </form>
  </section>

  {#if proposal}
    <section class="proposal" aria-labelledby="proposal-heading">
      <div class="proposal-heading">
        <div>
          <span class="eyebrow accent-label">Proposition d’agent</span>
          <h2 id="proposal-heading">{proposal.title}</h2>
        </div>
        <span class="proposal-kind">ajout</span>
      </div>
      <div class="diff" aria-label="Diff de la proposition">
        <p class="diff-line diff-context">  {fileName}</p>
        <p class="diff-line diff-add">+ {proposal.after}</p>
      </div>
      <p class="proposal-note">Rien ne change avant votre décision.</p>
      <div class="proposal-actions">
        <button class="secondary-button" type="button" on:click={rejectProposal}>Refuser</button>
        <button class="primary-button" type="button" on:click={acceptProposal}>Accepter</button>
      </div>
    </section>
  {/if}

  <footer class="status-bar" role="status" aria-live="polite">
    <span class:is-dirty={dirty} class="status-dot" aria-hidden="true"></span>
    <span>{notice}</span>
  </footer>
</main>
