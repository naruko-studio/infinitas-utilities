<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import {
    captureFrame,
    getConfig,
    getServerInfo,
    getStatus,
    onStatus,
    setDevCapture,
    setLanAccess,
    setLanguage,
    type Judge,
    type ServerInfo,
    type Status,
  } from "./lib/api";
  import Annotator from "./lib/Annotator.svelte";
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
  let server = $state<ServerInfo | null>(null);
  let tab = $state<"main" | "annotator">("main");
  let lanBusy = $state(false);

  /** OBS pages with the browser-source size that fits them at default styling. */
  const OBS_PAGES = [
    { path: "/obs/counter.html", size: "400 × 100" },
    { path: "/obs/judge.html", size: "300 × 270" },
  ] as const;

  $effect(() => {
    getVersion().then((v) => (version = v));
    // The server starts right after the window; ask again shortly.
    getServerInfo().then((s) => (server = s));
    const retry = setTimeout(() => getServerInfo().then((s) => (server = s)), 1500);
    getConfig().then((config) => {
      language = config.language;
      devCapture = config.capture.dev_capture;
      applyLanguage(language);
    });
    getStatus().then((s) => (status = s));
    const unlisten = onStatus((s) => (status = s));
    return () => {
      clearTimeout(retry);
      unlisten.then((f) => f());
    };
  });

  async function onLanChange(e: Event) {
    const enabled = (e.currentTarget as HTMLInputElement).checked;
    lanBusy = true;
    error = "";
    try {
      server = await setLanAccess(enabled);
    } catch (err) {
      error = t("error.configSave", { error: String(err) });
    } finally {
      lanBusy = false;
    }
  }

  async function copy(text: string) {
    await navigator.clipboard.writeText(text);
    message = t("web.copied", { url: text });
  }

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

  {#if devCapture}
    <nav class="tabs">
      <button class:active={tab === "main"} onclick={() => (tab = "main")}>{t("tab.main")}</button>
      <button class:active={tab === "annotator"} onclick={() => (tab = "annotator")}>
        {t("tab.annotator")}
      </button>
    </nav>
  {/if}

  {#if tab === "annotator" && devCapture}
    <Annotator />
  {:else}
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

  {#if server}
    <section class="card web">
      <h2>{t("web.title")}</h2>
      {#if server.running}
        <div class="muted">{t("web.obs")}</div>
        <ul class="urls">
          {#each OBS_PAGES as page (page.path)}
            {@const url = server.localUrl + page.path}
            <li>
              <code>{url}</code>
              <button onclick={() => copy(url)}>{t("web.copy")}</button>
              <span class="muted small">{t("web.size", { size: page.size })}</span>
            </li>
          {/each}
        </ul>
        <label>
          <input
            type="checkbox"
            checked={server.lanAccess}
            disabled={lanBusy}
            onchange={onLanChange}
          />
          {t("web.lan")}
        </label>
        {#if server.lanAccess}
          {#if server.lanUrl}
            <ul class="urls">
              <li>
                <code>{server.lanUrl}/</code>
                <button onclick={() => copy(server!.lanUrl + "/")}>{t("web.copy")}</button>
              </li>
            </ul>
          {:else}
            <p class="hint">{t("web.noLanIp")}</p>
          {/if}
          <p class="hint">{t("web.lan.hint")}</p>
        {/if}
      {:else}
        <p class="error">{t("web.error", { port: server.port, error: server.error ?? "" })}</p>
      {/if}
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
  {/if}
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
  .small {
    font-size: 0.8rem;
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
  .web {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .urls {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .urls li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  code {
    background: #23262e;
    padding: 2px 6px;
    border-radius: 4px;
    user-select: all;
  }
  .tabs {
    display: flex;
    gap: 6px;
  }
  .tabs button.active {
    background: #2f5d8a;
    color: #fff;
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
