<script lang="ts">
  // Developer screen: label screenshots and file them for resgen and the
  // regression tests. Field names are the game's own (PG, SCORE…), so only
  // the surrounding UI is translated.
  import {
    annotatorCheck,
    annotatorDiscard,
    annotatorImage,
    annotatorList,
    annotatorLoad,
    annotatorSave,
    songList,
    type Chart,
    type DjLevel,
    type Label,
    type Lamp,
    type Screen,
    type Shot,
    type Song,
  } from "./api";
  import { t } from "./i18n/index.svelte";

  const SCREENS: Screen[] = ["play", "result", "select", "option", "other"];
  const LAYOUTS = [
    "sp-1p-graph-inner",
    "sp-1p-graph-outer",
    "sp-2p-graph-inner",
    "sp-2p-graph-outer",
    "dp-info-left",
    "dp-info-right",
  ];
  const CHARTS: Chart[] = ["SPB", "SPN", "SPH", "SPA", "SPL", "DPN", "DPH", "DPA", "DPL"];
  const LAMPS: Lamp[] = [
    "no-play",
    "failed",
    "assist-clear",
    "easy-clear",
    "clear",
    "hard-clear",
    "ex-hard-clear",
    "full-combo",
  ];
  const DJ: DjLevel[] = ["F", "E", "D", "C", "B", "A", "AA", "AAA"];
  const GAUGES = ["off", "assist-easy", "easy", "hard", "ex-hard"] as const;
  const FILTERS = ["todo", "all", "unsorted", "game", ...SCREENS] as const;

  let shots = $state<Shot[]>([]);
  let filter = $state<(typeof FILTERS)[number]>("todo");
  let selected = $state<Shot | null>(null);
  let imageUrl = $state("");
  let label = $state<Label | null>(null);
  let suggestion = $state<Label | null>(null);
  let problems = $state<string[]>([]);
  let message = $state("");
  let songs = $state<Song[]>([]);

  const visible = $derived(
    shots.filter((s) => {
      switch (filter) {
        case "all":
          return true;
        case "todo":
          return !s.labeled;
        case "game":
          return s.source === "game";
        default:
          return s.folder === filter;
      }
    }),
  );

  $effect(() => {
    refresh();
    songList().then((l) => (songs = l.songs));
  });

  // Re-check the label whenever a field changes.
  $effect(() => {
    const l = label ? $state.snapshot(label) : null;
    if (!l) {
      problems = [];
      return;
    }
    annotatorCheck(l as Label)
      .then((p) => (problems = p))
      .catch((e) => (problems = [String(e)]));
  });

  async function refresh() {
    shots = await annotatorList();
  }

  async function open(shot: Shot) {
    selected = shot;
    message = "";
    if (imageUrl) URL.revokeObjectURL(imageUrl);
    imageUrl = URL.createObjectURL(
      new Blob([await annotatorImage(shot.path)], { type: "image/png" }),
    );
    const loaded = await annotatorLoad(shot.path);
    suggestion = loaded.suggestion;
    const known = SCREENS.includes(shot.folder as Screen) ? (shot.folder as Screen) : null;
    label = loaded.label ?? loaded.suggestion ?? (known ? blank(known) : null);
  }

  function blank(screen: Screen): Label {
    switch (screen) {
      case "play":
        return {
          screen,
          layout: LAYOUTS[0],
          judge: { pg: 0, gr: 0, gd: 0, bd: 0, pr: 0, cb: 0 },
        };
      case "result":
        return {
          screen,
          side: "1p",
          chart: "SPN",
          level: 1,
          songId: null,
          title: "",
          notes: 0,
          lamp: "clear",
          bestLamp: null,
          djLevel: "A",
          bestDjLevel: null,
          score: 0,
          bestScore: null,
          missCount: 0,
          bestMissCount: null,
          judge: { pg: 0, gr: 0, gd: 0, bd: 0, pr: 0 },
          comboBreak: 0,
          fast: 0,
          slow: 0,
          dead: null,
          useOption: null,
        };
      case "select":
        return {
          screen,
          style: "sp",
          chart: "SPN",
          level: 1,
          songId: null,
          title: "",
          lamp: "clear",
          djLevel: "A",
          score: 0,
          missCount: 0,
        };
      case "option":
        return { screen, style: "sp", gauge: "off" };
      case "other":
        return { screen };
    }
  }

  function setScreen(screen: Screen) {
    if (label?.screen === screen) return;
    label = suggestion?.screen === screen ? suggestion : blank(screen);
  }

  /** Song picked by "title — artist": fills ID, level and notes. */
  function pickSong(text: string) {
    if (!label || (label.screen !== "result" && label.screen !== "select")) return;
    const song = songs.find((s) => `${s.title} — ${s.artist}` === text || s.title === text);
    label.title = song?.title ?? text;
    label.songId = song?.id ?? null;
    fillChart();
  }

  function fillChart() {
    if (!label || (label.screen !== "result" && label.screen !== "select")) return;
    const songId = label.songId;
    const chart = songs.find((s) => s.id === songId)?.charts[label.chart];
    if (!chart) return;
    label.level = chart.level;
    if (label.screen === "result" && chart.notes) label.notes = chart.notes;
  }

  async function save() {
    if (!selected || !label) return;
    const at = visible.findIndex((s) => s.path === selected!.path);
    try {
      await annotatorSave(selected.path, $state.snapshot(label) as Label);
      message = t("annotator.saved");
      await refresh();
      next(at);
    } catch (e) {
      message = String(e);
    }
  }

  async function discard() {
    if (!selected) return;
    const at = visible.findIndex((s) => s.path === selected!.path);
    try {
      await annotatorDiscard(selected.path);
      await refresh();
      next(at);
    } catch (e) {
      message = String(e);
    }
  }

  /** After the list refreshed: open what is now at the old position, or
   *  the one after it if the same image is still there (filter "all"). */
  function next(at: number) {
    let n = visible[Math.max(0, at)];
    if (n && n.name === selected?.name) n = visible[at + 1];
    if (n) open(n);
    else {
      selected = null;
      label = null;
      imageUrl = "";
    }
  }

  function onKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      save();
    }
  }

  /** `<input type=number>` value, or null when empty (fields shown as "—"). */
  const num = (v: string) => (v === "" ? null : Number(v));
</script>

<svelte:window onkeydown={onKey} />

<div class="annotator">
  <aside class="list">
    <div class="bar">
      <select bind:value={filter}>
        {#each FILTERS as f (f)}
          <option value={f}>{t(`annotator.filter.${f}`)}</option>
        {/each}
      </select>
      <button onclick={refresh}>↻</button>
      <span class="muted">{visible.length}</span>
    </div>
    <ul>
      {#each visible as s (s.path)}
        <li>
          <button class:active={selected?.path === s.path} onclick={() => open(s)}>
            <span class="tag">{s.source === "game" ? "F12" : s.folder}</span>
            {#if s.labeled}<span class="ok">✓</span>{/if}
            <span class="name">{s.name}</span>
          </button>
        </li>
      {/each}
    </ul>
  </aside>

  <section class="view">
    {#if imageUrl}
      <img src={imageUrl} alt={selected?.name} />
    {:else}
      <p class="muted">{t("annotator.pick")}</p>
    {/if}
  </section>

  <aside class="form">
    {#if selected}
      <div class="muted small">{selected.width}×{selected.height} · {selected.name}</div>
      <div class="screens">
        {#each SCREENS as s (s)}
          <button class:active={label?.screen === s} onclick={() => setScreen(s)}>{s}</button>
        {/each}
      </div>
      {#if suggestion}
        <p class="hint">{t("annotator.suggested", { screen: suggestion.screen })}</p>
      {/if}

      {#if label?.screen === "play"}
        <label>layout
          <select bind:value={label.layout}>
            {#each LAYOUTS as l (l)}<option>{l}</option>{/each}
          </select>
        </label>
        <div class="grid">
          {#each ["pg", "gr", "gd", "bd", "pr", "cb"] as const as k (k)}
            <label>{k.toUpperCase()}<input type="number" min="0" bind:value={label.judge[k]} /></label>
          {/each}
        </div>
      {:else if label?.screen === "result" || label?.screen === "select"}
        <label>{t("annotator.song")}
          <input
            list="songs"
            value={label.title}
            onchange={(e) => pickSong(e.currentTarget.value)}
          />
        </label>
        <datalist id="songs">
          {#each songs as s (s.id)}<option value={`${s.title} — ${s.artist}`}></option>{/each}
        </datalist>
        {#if label.title && !label.songId}<p class="hint">{t("annotator.unknownSong")}</p>{/if}
        <div class="grid">
          {#if label.screen === "result"}
            <label>side
              <select bind:value={label.side}><option>1p</option><option>2p</option></select>
            </label>
          {:else}
            <label>style
              <select bind:value={label.style}><option>sp</option><option>dp</option></select>
            </label>
          {/if}
          <label>chart
            <select bind:value={label.chart} onchange={fillChart}>
              {#each CHARTS as c (c)}<option>{c}</option>{/each}
            </select>
          </label>
          <label>level<input type="number" min="1" max="12" bind:value={label.level} /></label>
          {#if label.screen === "result"}
            <label>NOTES<input type="number" min="0" bind:value={label.notes} /></label>
          {/if}
          <label>lamp
            <select bind:value={label.lamp}>
              {#each LAMPS as l (l)}<option>{l}</option>{/each}
            </select>
          </label>
          <label>DJ LEVEL
            <select
              value={label.djLevel ?? ""}
              onchange={(e) => {
                if (label?.screen === "result" || label?.screen === "select")
                  (label as { djLevel: DjLevel | null }).djLevel =
                    (e.currentTarget.value || null) as DjLevel | null;
              }}
            >
              <option value="">—</option>
              {#each DJ as d (d)}<option>{d}</option>{/each}
            </select>
          </label>
          <label>SCORE<input type="number" min="0" bind:value={label.score} /></label>
          <label>MISS
            <input
              type="number"
              min="0"
              placeholder="—"
              value={label.missCount ?? ""}
              oninput={(e) => {
                if (label?.screen === "result" || label?.screen === "select")
                  label.missCount = num(e.currentTarget.value);
              }}
            />
          </label>
        </div>
        {#if label.screen === "result"}
          <div class="grid">
            {#each ["pg", "gr", "gd", "bd", "pr"] as const as k (k)}
              <label>{k.toUpperCase()}<input type="number" min="0" bind:value={label.judge[k]} /></label>
            {/each}
            <label>CB<input type="number" min="0" bind:value={label.comboBreak} /></label>
            <label>FAST<input type="number" min="0" bind:value={label.fast} /></label>
            <label>SLOW<input type="number" min="0" bind:value={label.slow} /></label>
          </div>
          <details>
            <summary>{t("annotator.best")}</summary>
            <div class="grid">
              <label>lamp
                <select
                  value={label.bestLamp ?? ""}
                  onchange={(e) => {
                    if (label?.screen === "result")
                      label.bestLamp = (e.currentTarget.value || null) as Lamp | null;
                  }}
                >
                  <option value="">—</option>
                  {#each LAMPS as l (l)}<option>{l}</option>{/each}
                </select>
              </label>
              <label>DJ LEVEL
                <select
                  value={label.bestDjLevel ?? ""}
                  onchange={(e) => {
                    if (label?.screen === "result")
                      label.bestDjLevel = (e.currentTarget.value || null) as DjLevel | null;
                  }}
                >
                  <option value="">—</option>
                  {#each DJ as d (d)}<option>{d}</option>{/each}
                </select>
              </label>
              <label>SCORE
                <input
                  type="number"
                  placeholder="—"
                  value={label.bestScore ?? ""}
                  oninput={(e) => {
                    if (label?.screen === "result") label.bestScore = num(e.currentTarget.value);
                  }}
                />
              </label>
              <label>MISS
                <input
                  type="number"
                  placeholder="—"
                  value={label.bestMissCount ?? ""}
                  oninput={(e) => {
                    if (label?.screen === "result")
                      label.bestMissCount = num(e.currentTarget.value);
                  }}
                />
              </label>
            </div>
          </details>
          <label>USE OPTION
            <input
              placeholder="—"
              value={label.useOption ?? ""}
              oninput={(e) => {
                if (label?.screen === "result")
                  label.useOption = e.currentTarget.value.trim() || null;
              }}
            />
          </label>
          <label class="inline">
            <input
              type="checkbox"
              checked={label.dead !== null}
              onchange={(e) => {
                if (label?.screen === "result")
                  label.dead = e.currentTarget.checked ? { note: 0, measure: 1 } : null;
              }}
            />
            DEAD POINT
          </label>
          {#if label.dead}
            <div class="grid">
              <label>NOTE<input type="number" min="0" bind:value={label.dead.note} /></label>
              <label>MEASURE<input type="number" min="0" bind:value={label.dead.measure} /></label>
            </div>
          {/if}
        {/if}
      {:else if label?.screen === "option"}
        <div class="grid">
          <label>style
            <select bind:value={label.style}><option>sp</option><option>dp</option></select>
          </label>
          <label>gauge
            <select bind:value={label.gauge}>
              {#each GAUGES as g (g)}<option>{g}</option>{/each}
            </select>
          </label>
        </div>
      {/if}

      {#if problems.length}
        <ul class="problems">
          {#each problems as p (p)}<li>{p}</li>{/each}
        </ul>
      {:else if label}
        <p class="ok">{t("annotator.consistent")}</p>
      {/if}

      <div class="actions">
        <button class="primary" disabled={!label} onclick={save}>{t("annotator.save")}</button>
        {#if selected.source === "devCapture"}
          <button onclick={discard}>{t("annotator.discard")}</button>
        {/if}
      </div>
      {#if message}<p class="muted">{message}</p>{/if}
    {/if}
  </aside>
</div>

<style>
  .annotator {
    display: grid;
    grid-template-columns: 240px 1fr 320px;
    gap: 10px;
    height: calc(100vh - 120px);
    min-height: 420px;
  }
  .list,
  .form {
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: auto;
  }
  .bar {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .list li button {
    width: 100%;
    text-align: left;
    display: flex;
    gap: 6px;
    align-items: center;
    background: none;
    border: none;
    color: inherit;
    padding: 3px 4px;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .list li button.active,
  .screens button.active {
    background: #2f5d8a;
  }
  .tag {
    font-size: 0.7rem;
    color: #9aa0ad;
    min-width: 52px;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .view {
    overflow: auto;
    background: #111;
    border-radius: 6px;
    display: flex;
    align-items: flex-start;
    justify-content: center;
  }
  .view img {
    max-width: 100%;
    height: auto;
  }
  .screens {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px 8px;
  }
  label {
    display: flex;
    flex-direction: column;
    font-size: 0.75rem;
    color: #9aa0ad;
    gap: 2px;
  }
  label.inline {
    flex-direction: row;
    align-items: center;
    gap: 6px;
  }
  input,
  select {
    font-size: 0.85rem;
    min-width: 0;
  }
  .problems {
    color: #ff8a80;
    font-size: 0.8rem;
    padding-left: 16px;
    list-style: disc;
  }
  .ok {
    color: #81c784;
  }
  .small {
    font-size: 0.75rem;
  }
  .muted,
  .hint {
    color: #9aa0ad;
    font-size: 0.8rem;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .primary {
    font-weight: 700;
  }
</style>
