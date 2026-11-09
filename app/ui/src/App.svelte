<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import {
    captureFrame,
    getConfig,
    getStatus,
    onStatus,
    setDevCapture,
    setLanguage,
    type Judge,
    type Status,
  } from "./lib/api";
  import {
    applyLanguage,
    LOCALE_NAMES,
    LOCALES,
    t,
    type LanguageSetting,
  } from "./lib/i18n/index.svelte";

  const JUDGE_KEYS = ["pg", "gr", "gd", "bd", "pr", "cb"] as const;

  let version = $state("");
  let language = $state<LanguageSetting>("auto");
  let devCapture = $state(false);
  let status = $state<Status | null>(null);
  let message = $state("");
  let error = $state("");

  $effect(() => {
    getVersion().then((v) => (version = v));
    getConfig().then((config) => {
      language = config.language;
      devCapture = config.capture.dev_capture;
      applyLanguage(language);
    });
    getStatus().then((s) => (status = s));
    const unlisten = onStatus((s) => (status = s));
    return () => {
      unlisten.then((f) => f());
    };
  });

  const connectionText = $derived.by(() => {
    if (!status) return "";
    const [width, height] = status.resolution ?? [0, 0];
    const params = { sender: status.sender, width, height, detail: status.detail ?? "" };
    return t(`connection.${status.connection}`, params);
  });

  async function save(action: () => Promise<void>) {
    error = "";
    try {
      await action();
    } catch (e) {
      error = t("error.configSave", { error: String(e) });
    }
  }

  function onLanguageChange() {
    applyLanguage(language);
    save(() => setLanguage(language));
  }

  function onDevCaptureChange() {
    save(() => setDevCapture(devCapture));
  }

  async function onCapture() {
    try {
      message = t("capture.saved", { path: await captureFrame() });
    } catch (e) {
      message = t("capture.failed", { error: String(e) });
    }
  }

  const notes = (j: Judge) => j.pg + j.gr + j.gd + j.bd;
</script>

<main>
  <header>
    <h1>{t("app.title")}</h1>
    {#if version}<span class="muted">{t("app.version", { version })}</span>{/if}
  </header>

  {#if status}
    <p class="connection {status.connection}">{connectionText}</p>

    <section class="cards">
      <div class="card">
        <h2>{t("today.title")}</h2>
        <div class="big">{status.todayNotes.toLocaleString()}</div>
        <div class="muted">
          {t("today.notes")} · {t("today.plays")} {status.playsToday}
        </div>
        {@render judgeTable(status.today)}
      </div>

      <div class="card">
        <h2>{t("current.title")}</h2>
        {#if status.current && status.layout}
          <div class="big">{notes(status.current).toLocaleString()}</div>
          <div class="muted">{t("play.onScreen", { layout: status.layout })}</div>
          {@render judgeTable(status.current)}
        {:else if status.lastPlay}
          <div class="big">{notes(status.lastPlay.judge).toLocaleString()}</div>
          <div class="muted">{t("play.finished", { layout: status.lastPlay.layout })}</div>
          {@render judgeTable(status.lastPlay.judge)}
        {:else}
          <div class="muted">{t("play.notOnScreen")}</div>
          {#if status.connection === "connected"}
            <p class="hint">{t("play.hint")}</p>
          {/if}
        {/if}
      </div>
    </section>
  {/if}

  <section class="settings">
    <label>
      {t("settings.language")}
      <select bind:value={language} onchange={onLanguageChange}>
        <option value="auto">{t("settings.language.auto")}</option>
        {#each LOCALES as code (code)}
          <option value={code}>{LOCALE_NAMES[code]}</option>
        {/each}
      </select>
    </label>

    <label title={t("settings.devCapture.hint")}>
      <input type="checkbox" bind:checked={devCapture} onchange={onDevCaptureChange} />
      {t("settings.devCapture")}
    </label>
    {#if devCapture}
      <button onclick={onCapture}>{t("capture.button")}</button>
    {/if}
  </section>

  {#if message}<p class="muted">{message}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</main>

{#snippet judgeTable(j: Judge)}
  <table>
    <tbody>
      {#each JUDGE_KEYS as key (key)}
        <tr>
          <th class={key}>{key.toUpperCase()}</th>
          <td>{j[key].toLocaleString()}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/snippet}

<style>
  main {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  h1 {
    font-size: 1.3rem;
    margin: 0;
  }
  h2 {
    font-size: 0.95rem;
    margin: 0 0 6px;
    color: #9aa0ad;
    font-weight: 600;
  }
  .muted,
  .hint {
    color: #9aa0ad;
  }
  .hint {
    font-size: 0.85rem;
  }
  .connection {
    margin: 0;
    padding: 8px 12px;
    border-radius: 6px;
    background: #23262e;
  }
  .connection.connected {
    border-left: 4px solid #4caf50;
  }
  .connection.waiting,
  .connection.starting {
    border-left: 4px solid #ffb300;
  }
  .connection.error,
  .connection.unsupportedResolution {
    border-left: 4px solid #ef5350;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 12px;
  }
  .card {
    background: #1e2027;
    border-radius: 8px;
    padding: 12px 16px;
  }
  .big {
    font-size: 2rem;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  table {
    margin-top: 8px;
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
  }
  th {
    text-align: left;
    padding-right: 16px;
    font-weight: 600;
  }
  td {
    text-align: right;
    min-width: 60px;
  }
  th.pg {
    color: #4dd0e1;
  }
  th.gr {
    color: #fff176;
  }
  th.gd {
    color: #ffb300;
  }
  th.bd {
    color: #ff9800;
  }
  th.pr {
    color: #ef5350;
  }
  th.cb {
    color: #ff8a80;
  }
  .settings {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    align-items: center;
  }
  label {
    display: inline-flex;
    gap: 8px;
    align-items: center;
  }
  .error {
    color: #ff8a80;
  }
</style>
