<script lang="ts">
  // Results whose title was not recognized: the user picks the song once and
  // the engine learns that title for next time.
  import {
    LAMP_NAMES,
    nameResult,
    onResultChange,
    songList,
    unnamedResults,
    type Candidate,
    type UnnamedResult,
  } from "./api";
  import { t } from "./i18n/index.svelte";

  let pending = $state<UnnamedResult[]>([]);
  let busy = $state(false);
  let error = $state("");
  /** Search text per result, for results without candidates. */
  let search = $state<Record<number, string>>({});
  let allSongs = $state<Candidate[] | null>(null);

  async function refresh() {
    try {
      pending = await unnamedResults();
    } catch {
      pending = [];
    }
  }

  $effect(() => {
    refresh();
    const unlisten = onResultChange(refresh);
    return () => {
      unlisten.then((f) => f());
    };
  });

  async function pick(id: number | null, songId: string) {
    if (id === null) return;
    busy = true;
    error = "";
    try {
      await nameResult(id, songId);
      await refresh();
    } catch (e) {
      error = t("unnamed.failed", { error: String(e) });
    } finally {
      busy = false;
    }
  }

  async function matches(text: string): Promise<Candidate[]> {
    allSongs ??= (await songList()).songs.map((s) => ({
      id: s.id,
      title: s.title,
      artist: s.artist,
    }));
    const q = text.trim().toLowerCase();
    if (!q) return [];
    return allSongs.filter((s) => s.title.toLowerCase().includes(q)).slice(0, 12);
  }
</script>

{#if pending.length > 0}
  <section class="card unnamed">
    <h2>{t("unnamed.title")}</h2>
    <p class="hint">{t("unnamed.hint")}</p>
    {#each pending as p (p.result.id)}
      {@const r = p.result}
      <div class="item">
        <div class="what">
          <span class="muted">{new Date(r.recordedAt).toLocaleTimeString()}</span>
          {#if r.chart}<span>{r.chart}{r.level ?? ""}</span>{/if}
          <span>{r.notes} NOTES</span>
          {#if r.lamp}<span class="lamp {r.lamp}">{LAMP_NAMES[r.lamp]}</span>{/if}
          <span>SCORE {r.score}</span>
        </div>
        {#if p.candidates.length > 0}
          <div class="choices">
            {#each p.candidates as c (c.id)}
              <button disabled={busy} title={c.artist} onclick={() => pick(r.id, c.id)}>
                {c.title}
              </button>
            {/each}
          </div>
        {:else}
          <p class="hint">{t("unnamed.noCandidates")}</p>
          <input
            placeholder={t("unnamed.search")}
            value={search[r.id ?? 0] ?? ""}
            oninput={(e) => (search[r.id ?? 0] = e.currentTarget.value)}
          />
          {#await matches(search[r.id ?? 0] ?? "") then found}
            <div class="choices">
              {#each found as c (c.id)}
                <button disabled={busy} title={c.artist} onclick={() => pick(r.id, c.id)}>
                  {c.title}
                </button>
              {/each}
            </div>
          {/await}
        {/if}
      </div>
    {/each}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </section>
{/if}

<style>
  .card {
    background: #1e2027;
    border-radius: 8px;
    padding: 12px 16px;
  }
  h2 {
    font-size: 0.95rem;
    margin: 0 0 6px;
    color: #ffb300;
    font-weight: 600;
  }
  .hint,
  .muted {
    color: #9aa0ad;
  }
  .hint {
    font-size: 0.85rem;
    margin: 0 0 6px;
  }
  .item {
    border-top: 1px solid #2c2f38;
    padding: 8px 0;
  }
  .what {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    font-weight: 600;
  }
  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
  }
  .lamp.failed {
    color: #ef5350;
  }
  .lamp.full-combo {
    color: #4dd0e1;
  }
  .lamp.ex-hard-clear {
    color: #ffd54f;
  }
  .lamp.hard-clear {
    color: #ff8a80;
  }
  .lamp.easy-clear {
    color: #aed581;
  }
  .lamp.clear {
    color: #81d4fa;
  }
  .lamp.assist-clear {
    color: #ce93d8;
  }
  .error {
    color: #ff8a80;
  }
</style>
