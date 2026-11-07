<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { getConfig, setLanguage } from "./lib/api";
  import {
    applyLanguage,
    LOCALE_NAMES,
    LOCALES,
    t,
    type LanguageSetting,
  } from "./lib/i18n/index.svelte";

  let version = $state("");
  let language = $state<LanguageSetting>("auto");
  let error = $state("");

  $effect(() => {
    getVersion().then((v) => (version = v));
    getConfig().then((config) => {
      language = config.language;
      applyLanguage(language);
    });
  });

  async function onLanguageChange() {
    applyLanguage(language);
    error = "";
    try {
      await setLanguage(language);
    } catch (e) {
      error = t("error.configSave", { error: String(e) });
    }
  }
</script>

<main>
  <header>
    <h1>{t("app.title")}</h1>
    {#if version}<span class="version">{t("app.version", { version })}</span>{/if}
  </header>

  <label>
    {t("settings.language")}
    <select bind:value={language} onchange={onLanguageChange}>
      <option value="auto">{t("settings.language.auto")}</option>
      {#each LOCALES as code (code)}
        <option value={code}>{LOCALE_NAMES[code]}</option>
      {/each}
    </select>
  </label>

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <p class="muted">{t("status.notImplemented")}</p>
</main>

<style>
  main {
    padding: 16px 20px;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  h1 {
    font-size: 1.3rem;
    margin: 0 0 16px;
  }
  .version,
  .muted {
    color: #9aa0ad;
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
