<script lang="ts">
  import { onMount } from 'svelte';
  import {
    AppWindow, ArrowDown, ArrowUp, Check, Database, GripVertical, PanelsTopLeft, RotateCcw, Download, Expand, Info, Languages, LoaderCircle, Monitor, Moon, Palette, Plus, Sun, SwatchBook,
    CalendarDays, PanelRight, Pipette, Trash, TriangleAlert, Upload, X, ZoomIn,
  } from '@lucide/svelte';
  import { calPrefs, setCalPrefs } from '../lib/calPrefs.svelte';
  import { loadPanelWidgets, PANEL_WIDGETS, panelApi, savePanelWidgets, type Corner, type PanelStatus } from '../lib/panel';
  import { WIDGETS, type WidgetId } from '../lib/home.svelte';
  import { broadcastPrefs } from '../lib/sync.svelte';
  import Modal from '../components/Modal.svelte';
  import { api, type BackupManifest } from '../lib/api';
  import { backupError, exportBackup, pickBackup } from '../lib/backup';
  import { fmtTs } from '../lib/dates';
  import { i18n, setLocale, t, type Locale } from '../lib/i18n.svelte';
  import { ACCENTS, setAccent, setCustomTheme, setTheme, toast, ui, type PresetAccent, type ThemePref } from '../lib/state.svelte';
  import { fromHex, hex } from '../lib/colors';
  import { DEFAULT_CUSTOM_THEME, TINT_LEVELS } from '../lib/customTheme';
  import {
    MAX_SAVED, ZOOM_STEPS, applyPreset, applySaved, currentSize, loadSavedSizes, maximize, setZoom, storeSavedSizes, zoom,
    type WindowSize,
  } from '../lib/window.svelte';
  import { shortcut } from '../lib/clipboard.svelte';
  import { movePage, orderedPages, pagePrefs, resetPages, setPageVisible, visiblePages } from '../lib/pages.svelte';

  const LANGUAGES: { id: Locale; name: string; sample: string; badge: string }[] = [
    { id: 'ko', name: '한국어', sample: '안녕하세요! 오늘 일정을 확인해 볼까요?', badge: '가' },
    { id: 'en', name: 'English', sample: "Hello! Let's check today's schedule.", badge: 'A' },
    { id: 'ja', name: '日本語', sample: 'こんにちは！今日の予定を確認しましょう。', badge: 'あ' },
  ];

  const THEMES: { id: ThemePref; icon: typeof Sun; label: () => string }[] = [
    { id: 'light', icon: Sun, label: () => t('set.light') },
    { id: 'dark', icon: Moon, label: () => t('set.dark') },
    { id: 'system', icon: Monitor, label: () => t('set.system') },
  ];

  /** Preview colors per theme: main accent, soft tint, second accent. */
  const ACCENT_PREVIEW: Record<PresetAccent, [string, string, string]> = {
    default: ['#6c63ff', '#eeedff', '#b084f6'],
    mono: ['#1c1c1f', '#ececef', '#8a8a93'],
    pink: ['#e2558f', '#fdebf2', '#ffa98a'],
    blue: ['#2f7cf6', '#e7f0fe', '#4cc9f0'],
    green: ['#1f9d68', '#e3f5ec', '#9bd35a'],
    brown: ['#a8691f', '#faefd9', '#f2b933'],
  };

  // ---- overlay panel (Windows desktop only)
  const inTauri = '__TAURI_INTERNALS__' in window;
  let panel = $state<PanelStatus | null>(null);
  let panelWidgets = $state<WidgetId[]>([]);
  const CORNERS: { id: Corner; label: () => string }[] = [
    { id: 'top-left', label: () => t('panel.topLeft') },
    { id: 'top-right', label: () => t('panel.topRight') },
    { id: 'bottom-left', label: () => t('panel.bottomLeft') },
    { id: 'bottom-right', label: () => t('panel.bottomRight') },
  ];

  async function refreshPanel() {
    panel = await panelApi.status().catch(() => null);
  }

  async function panelAction(work: Promise<unknown>) {
    try {
      await work;
    } catch (e) {
      toast(String(e), 'error');
    }
    await refreshPanel();
  }

  async function togglePanelWidget(id: WidgetId, on: boolean) {
    panelWidgets = on ? [...panelWidgets, id] : panelWidgets.filter((x) => x !== id);
    await savePanelWidgets(panelWidgets).catch(() => {});
    broadcastPrefs();
  }

  onMount(() => {
    if (!inTauri) return;
    refreshPanel();
    loadPanelWidgets().then((w) => (panelWidgets = w));
  });

  const PRESETS = [0.6, 0.7, 0.8, 0.9];
  let saved = $state<WindowSize[]>([]);
  let size = $state(currentSize());

  onMount(() => {
    loadSavedSizes().then((s) => (saved = s));
  });

  async function saveCurrent() {
    if (saved.length >= MAX_SAVED) return;
    saved = [...saved, currentSize()];
    await storeSavedSizes(saved);
    toast(t('set.sizeSaved'), 'success');
  }

  async function removeSaved(i: number) {
    saved = saved.filter((_, j) => j !== i);
    await storeSavedSizes(saved);
  }

  const run = (p: Promise<unknown>) => p.catch((e) => toast(String(e), 'error'));

  // ---- pages: drag a row (or use the arrows) to reorder
  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);

  function dropPage(to: number) {
    if (dragFrom !== null) movePage(dragFrom, to);
    dragFrom = dragOver = null;
  }

  // ---- backup / restore / reset
  let exporting = $state(false);
  let importing = $state<{ path: string; manifest: BackupManifest } | null>(null);
  let resetOpen = $state(false);
  let resetUnderstood = $state(false);
  let restarting = $state(false);

  const COUNT_KEYS = ['events', 'todos', 'memos', 'ddays', 'bookmarks', 'expenses', 'meals', 'workouts', 'trpg', 'books', 'images'] as const;

  async function doExport() {
    exporting = true;
    await exportBackup();
    exporting = false;
  }

  async function startImport() {
    importing = await pickBackup();
  }

  async function confirmImport() {
    if (!importing) return;
    restarting = true;
    try {
      await api.importData(importing.path); // the app restarts on success
    } catch (e) {
      toast(backupError(e), 'error');
    }
    restarting = false;
  }

  async function confirmReset() {
    if (!resetUnderstood) return;
    restarting = true;
    try {
      await api.resetAllData(); // the app restarts on success
    } catch (e) {
      toast(String(e), 'error');
    }
    restarting = false;
  }
</script>

<svelte:window onresize={() => (size = currentSize())} />

<div class="page">
  <div class="page-header">
    <div>
      <h1>{t('nav.settings')}</h1>
      <p class="sub">{t('set.subtitle')}</p>
    </div>
  </div>

  <div class="sections">
    <section class="card">
      <div class="section-head">
        <div class="icon"><Languages size={18} /></div>
        <div>
          <h2>{t('set.language')}</h2>
          <p class="muted small">{t('set.languageBody')}</p>
        </div>
      </div>
      <div class="options two">
        {#each LANGUAGES as lang (lang.id)}
          <button class="option" class:selected={i18n.locale === lang.id} onclick={() => setLocale(lang.id)}>
            <span class="badge">{lang.badge}</span>
            <span class="text">
              <span class="name">{lang.name}</span>
              <span class="muted small">{lang.sample}</span>
            </span>
            <span class="check-mark">{#if i18n.locale === lang.id}<Check size={14} strokeWidth={3} />{/if}</span>
          </button>
        {/each}
      </div>
    </section>

    <section class="card">
      <div class="section-head">
        <div class="icon"><Palette size={18} /></div>
        <div>
          <h2>{t('set.appearance')}</h2>
          <p class="muted small">{t('set.appearanceBody')}</p>
        </div>
      </div>
      <div class="options three">
        {#each THEMES as theme (theme.id)}
          <button class="option theme" class:selected={ui.theme === theme.id} onclick={() => setTheme(theme.id)}>
            <span class="preview {theme.id}">
              <span class="pv-side"></span>
              <span class="pv-main"><span class="pv-line"></span><span class="pv-line short"></span><span class="pv-card"></span></span>
            </span>
            <span class="theme-label">
              <theme.icon size={15} />
              {theme.label()}
              <span class="check-mark">{#if ui.theme === theme.id}<Check size={14} strokeWidth={3} />{/if}</span>
            </span>
          </button>
        {/each}
      </div>
    </section>

    <section class="card">
      <div class="section-head">
        <div class="icon"><SwatchBook size={18} /></div>
        <div>
          <h2>{t('set.colorTheme')}</h2>
          <p class="muted small">{t('set.colorThemeBody')}</p>
        </div>
      </div>
      <div class="options accents">
        {#each ACCENTS as accent (accent)}
          {@const [main, soft, second] = ACCENT_PREVIEW[accent]}
          <button class="option accent" class:selected={ui.accent === accent} onclick={() => setAccent(accent)}>
            <span class="swatch" style:background="linear-gradient(140deg, {main}, {second})">
              <span class="swatch-soft" style:background={soft}></span>
            </span>
            <span class="name">{t(`accent.${accent}`)}</span>
            <span class="check-mark">{#if ui.accent === accent}<Check size={14} strokeWidth={3} />{/if}</span>
          </button>
        {/each}
        <button class="option accent" class:selected={ui.accent === 'custom'} onclick={() => setCustomTheme(ui.customTheme)}>
          <span class="swatch" style:background="linear-gradient(140deg, {hex(ui.customTheme.primary)}, {hex(ui.customTheme.accent2)})">
            <span class="swatch-soft custom-badge"><Pipette size={10} /></span>
          </span>
          <span class="name">{t('accent.custom')}</span>
          <span class="check-mark">{#if ui.accent === 'custom'}<Check size={14} strokeWidth={3} />{/if}</span>
        </button>
      </div>

      {#if ui.accent === 'custom'}
        <div class="custom-editor">
          <label class="color-field">
            <input type="color" value={hex(ui.customTheme.primary)} oninput={(e) => setCustomTheme({ ...ui.customTheme, primary: fromHex(e.currentTarget.value) })} />
            <span>
              <span class="s-title">{t('set.customPrimary')}</span>
              <span class="muted small">{t('set.customPrimaryBody')}</span>
            </span>
          </label>
          <label class="color-field">
            <input type="color" value={hex(ui.customTheme.accent2)} oninput={(e) => setCustomTheme({ ...ui.customTheme, accent2: fromHex(e.currentTarget.value) })} />
            <span>
              <span class="s-title">{t('set.customAccent2')}</span>
              <span class="muted small">{t('set.customAccent2Body')}</span>
            </span>
          </label>
          <div class="tint-row">
            <span class="s-title">{t('set.customTint')}</span>
            <div class="segmented">
              {#each TINT_LEVELS as level (level)}
                <button class:active={ui.customTheme.tint === level} onclick={() => setCustomTheme({ ...ui.customTheme, tint: level })}>
                  {t(`set.tint${level}`)}
                </button>
              {/each}
            </div>
          </div>
          <div class="custom-preview">
            <button class="btn primary small" tabindex="-1">{t('set.customSample')}</button>
            <span class="chip-soft">{t('common.today')}</span>
            <span class="bar-sample"></span>
            <button class="btn small ghost" onclick={() => setCustomTheme({ ...DEFAULT_CUSTOM_THEME })}>{t('set.customReset')}</button>
          </div>
        </div>
      {/if}
    </section>

    <section class="card">
      <div class="section-head">
        <div class="icon"><CalendarDays size={18} /></div>
        <div>
          <h2>{t('set.calendar')}</h2>
          <p class="muted small">{t('set.calendarBody')}</p>
        </div>
      </div>
      <label class="cal-opt">
        <span>
          <span class="cal-opt-title">{t('set.allDayLast')}</span>
          <span class="muted small">{t('set.allDayLastBody')}</span>
        </span>
        <input type="checkbox" class="switch" checked={calPrefs.allDayLast} onchange={(e) => setCalPrefs({ allDayLast: e.currentTarget.checked })} />
      </label>
      <div class="cal-opt">
        <span>
          <span class="cal-opt-title">{t('set.timePicker')}</span>
          <span class="muted small">{t('set.timePickerBody')}</span>
        </span>
        <div class="segmented">
          <button class:active={calPrefs.timePicker === 'list'} onclick={() => setCalPrefs({ timePicker: 'list' })}>{t('set.timePickerList')}</button>
          <button class:active={calPrefs.timePicker === 'precise'} onclick={() => setCalPrefs({ timePicker: 'precise' })}>{t('set.timePickerPrecise')}</button>
        </div>
      </div>
    </section>

    {#if panel}
      <section class="card">
        <div class="section-head">
          <div class="icon"><PanelRight size={18} /></div>
          <div>
            <h2>{t('panel.title')}</h2>
            <p class="muted small">{t('panel.body')}</p>
          </div>
        </div>
        <label class="cal-opt">
          <span>
            <span class="cal-opt-title">{t('panel.show')}</span>
            <span class="muted small">{t('panel.showBody')}</span>
          </span>
          <input type="checkbox" class="switch" checked={panel.open} onchange={(e) => panelAction(panelApi.setEnabled(e.currentTarget.checked))} />
        </label>
        <div class="cal-opt">
          <span>
            <span class="cal-opt-title">{t('panel.position')}</span>
            <span class="muted small">{t('panel.positionBody')}</span>
          </span>
          <div class="segmented">
            {#each CORNERS as c (c.id)}
              <button disabled={!panel.open} onclick={() => panelAction(panelApi.place(c.id))}>{c.label()}</button>
            {/each}
          </div>
        </div>
        <label class="cal-opt">
          <span>
            <span class="cal-opt-title">{t('panel.onTop')}</span>
            <span class="muted small">{t('panel.onTopBody')}</span>
          </span>
          <input type="checkbox" class="switch" checked={panel.on_top} onchange={(e) => panelAction(panelApi.setOnTop(e.currentTarget.checked))} />
        </label>
        <label class="cal-opt">
          <span>
            <span class="cal-opt-title">{t('panel.autostart')}</span>
            <span class="muted small">{t('panel.autostartBody')}</span>
          </span>
          <input type="checkbox" class="switch" checked={panel.autostart} onchange={(e) => panelAction(panelApi.setAutostart(e.currentTarget.checked))} />
        </label>
        <div class="cal-opt panel-widgets">
          <span>
            <span class="cal-opt-title">{t('panel.widgets')}</span>
            <span class="muted small">{t('panel.widgetsBody')}</span>
          </span>
          <div class="widget-picks">
            {#each PANEL_WIDGETS as id (id)}
              <label class="widget-pick">
                <input type="checkbox" checked={panelWidgets.includes(id)} onchange={(e) => togglePanelWidget(id, e.currentTarget.checked)} />
                {t(WIDGETS[id].name)}
              </label>
            {/each}
          </div>
        </div>
      </section>
    {/if}

    <section class="card">
      <div class="section-head">
        <div class="icon"><PanelsTopLeft size={18} /></div>
        <div>
          <h2>{t('set.pages')}</h2>
          <p class="muted small">{t('set.pagesBody')}</p>
        </div>
        <span class="spacer"></span>
        <button class="btn small ghost" onclick={resetPages}><RotateCcw size={14} /> {t('set.pagesReset')}</button>
      </div>
      <div class="page-list" role="list">
        {#each orderedPages() as page, i (page.id)}
          {@const shown = !pagePrefs.hidden.includes(page.id)}
          <div
            class="page-row"
            class:off={!shown}
            class:dragging={dragFrom === i}
            class:drop-target={dragOver === i && dragFrom !== i}
            draggable="true"
            ondragstart={(e) => { dragFrom = i; e.dataTransfer?.setData('text/plain', page.id); }}
            ondragover={(e) => { e.preventDefault(); dragOver = i; }}
            ondrop={(e) => { e.preventDefault(); dropPage(i); }}
            ondragend={() => (dragFrom = dragOver = null)}
            role="listitem"
          >
            <span class="grip"><GripVertical size={16} /></span>
            <span class="page-icon"><page.icon size={17} /></span>
            <span class="page-name">{t(page.label)}</span>
            {#if !shown}<span class="faint small">{t('set.pageHidden')}</span>{/if}
            <span class="spacer"></span>
            <button class="icon-btn" onclick={() => movePage(i, i - 1)} disabled={i === 0} aria-label={t('set.pageUp')} title={t('set.pageUp')}><ArrowUp size={15} /></button>
            <button class="icon-btn" onclick={() => movePage(i, i + 1)} disabled={i === pagePrefs.order.length - 1} aria-label={t('set.pageDown')} title={t('set.pageDown')}><ArrowDown size={15} /></button>
            <input
              type="checkbox"
              class="switch"
              checked={shown}
              disabled={shown && visiblePages().length <= 1}
              onchange={(e) => setPageVisible(page.id, e.currentTarget.checked)}
              aria-label={t('set.pageShow', { page: t(page.label) })}
            />
          </div>
        {/each}
      </div>
    </section>

    <section class="card">
      <div class="section-head">
        <div class="icon"><ZoomIn size={18} /></div>
        <div>
          <h2>{t('set.zoom')}</h2>
          <p class="muted small">{t('set.zoomBody', { keys: `${shortcut('+')} / ${shortcut('-')}` })}</p>
        </div>
      </div>
      <div class="zooms">
        {#each ZOOM_STEPS as z (z)}
          <button class="zoom" class:selected={Math.abs(zoom.level - z) < 0.001} onclick={() => run(setZoom(z))}>
            <span class="zoom-sample" style:font-size="{Math.round(15 * z)}px">Aa</span>
            {Math.round(z * 100)}%
          </button>
        {/each}
      </div>
      {#if zoom.level !== 1}
        <button class="btn small zoom-reset" onclick={() => run(setZoom(1))}>{t('set.zoomReset')}</button>
      {/if}
    </section>

    <section class="card">
      <div class="section-head">
        <div class="icon"><AppWindow size={18} /></div>
        <div>
          <h2>{t('set.window')}</h2>
          <p class="muted small">{t('set.windowBody')} · {t('set.current', { w: size.width, h: size.height })}</p>
        </div>
      </div>

      <div class="label sub-label">{t('set.presets')}</div>
      <div class="presets">
        {#each PRESETS as p (p)}
          <button class="preset" onclick={() => run(applyPreset(p))} title={t('set.presetHint', { pct: `${p * 100}%` })}>
            <span class="screen"><span class="win" style:width="{p * 100}%" style:height="{p * 100}%"></span></span>
            {p * 100}%
          </button>
        {/each}
        <button class="preset" onclick={() => run(maximize())}>
          <span class="screen"><span class="win" style:width="100%" style:height="100%"></span></span>
          <span class="row"><Expand size={13} /> {t('set.maximize')}</span>
        </button>
      </div>

      <div class="label sub-label">{t('set.saved')}</div>
      <div class="saved">
        {#each saved as s, i (i)}
          <div class="saved-item">
            <span class="saved-name">{t('set.sizeName', { n: i + 1 })}</span>
            <span class="muted small tabular">{s.width} × {s.height}</span>
            <span class="spacer"></span>
            <button class="btn small" onclick={() => run(applySaved(s))}>{t('set.apply')}</button>
            <button class="icon-btn danger" onclick={() => removeSaved(i)} aria-label={t('common.delete')}><X size={15} /></button>
          </div>
        {:else}
          <p class="muted small">{t('set.noSaved')}</p>
        {/each}
      </div>
      <div class="row save-row">
        <button class="btn" onclick={saveCurrent} disabled={saved.length >= MAX_SAVED}><Plus size={16} /> {t('set.saveCurrent')}</button>
        {#if saved.length >= MAX_SAVED}<span class="faint small">{t('set.savedFull')}</span>{/if}
      </div>
    </section>

    <section class="card">
      <div class="section-head">
        <div class="icon"><Database size={18} /></div>
        <div>
          <h2>{t('data.title')}</h2>
          <p class="muted small">{t('data.body')}</p>
        </div>
      </div>
      <div class="data-actions">
        <button class="data-action" onclick={doExport} disabled={exporting}>
          <span class="da-icon">{#if exporting}<span class="spin"><LoaderCircle size={20} /></span>{:else}<Download size={20} />{/if}</span>
          <span class="da-text">
            <span class="strong">{exporting ? t('data.exporting') : t('data.export')}</span>
            <span class="muted small">{t('data.exportHint')}</span>
          </span>
        </button>
        <button class="data-action" onclick={startImport}>
          <span class="da-icon"><Upload size={20} /></span>
          <span class="da-text">
            <span class="strong">{t('data.import')}</span>
            <span class="muted small">{t('data.importHint')}</span>
          </span>
        </button>
      </div>
    </section>

    <section class="card danger-zone">
      <div class="section-head">
        <div class="icon danger-icon"><TriangleAlert size={18} /></div>
        <div>
          <h2>{t('data.danger')}</h2>
          <p class="muted small">{t('data.resetHint')}</p>
        </div>
        <span class="spacer"></span>
        <button class="btn danger-solid" onclick={() => { resetUnderstood = false; resetOpen = true; }}>
          <Trash size={16} /> {t('data.reset')}
        </button>
      </div>
    </section>

    <section class="card about">
      <div class="section-head">
        <div class="icon"><Info size={18} /></div>
        <div>
          <h2>{t('set.about')}</h2>
          <p class="muted small">{t('set.aboutBody', { v: 'v0.1.0' })}</p>
        </div>
      </div>
    </section>
  </div>
</div>

{#if importing}
  <Modal title={t('data.importTitle')} onclose={() => !restarting && (importing = null)} width={480}>
    <p class="strong">{t('data.backupFrom', { date: fmtTs(importing.manifest.exported_at, { dateStyle: 'long', timeStyle: 'short' }) })}</p>
    <div class="field">
      <span class="label">{t('data.contains')}</span>
      <div class="counts">
        {#each COUNT_KEYS as key (key)}
          <span class="chip">{t(`data.count.${key}`, { n: importing.manifest.counts[key] ?? 0 })}</span>
        {/each}
      </div>
    </div>
    <div class="caution">
      <TriangleAlert size={18} />
      <p>{t('data.importWarning')}</p>
    </div>
    <button class="btn export-first" onclick={doExport} disabled={exporting || restarting}><Download size={15} /> {t('data.exportFirst')}</button>
    {#snippet footer()}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (importing = null)} disabled={restarting}>{t('common.cancel')}</button>
      <button class="btn primary" onclick={confirmImport} disabled={restarting}>
        {#if restarting}<span class="spin"><LoaderCircle size={16} /></span> {t('data.restarting')}{:else}<Upload size={16} /> {t('data.importConfirm')}{/if}
      </button>
    {/snippet}
  </Modal>
{/if}

{#if resetOpen}
  <Modal title={t('data.resetTitle')} onclose={() => !restarting && (resetOpen = false)} width={500}>
    <div class="caution danger">
      <TriangleAlert size={20} />
      <div>
        <p class="strong">{t('data.resetWarning')}</p>
        <ul>
          <li>{t('data.resetItem1')}</li>
          <li>{t('data.resetItem2')}</li>
          <li>{t('data.resetItem3')}</li>
          <li>{t('data.resetItem4')}</li>
        </ul>
      </div>
    </div>
    <p class="muted">{t('data.resetAdvice')}</p>
    <button class="btn export-first" onclick={doExport} disabled={exporting || restarting}><Download size={15} /> {t('data.exportFirst')}</button>
    <label class="understand">
      <input type="checkbox" class="check" bind:checked={resetUnderstood} />
      <span>{t('data.resetCheck')}</span>
    </label>
    {#snippet footer()}
      <span class="spacer"></span>
      <button class="btn ghost" onclick={() => (resetOpen = false)} disabled={restarting}>{t('common.cancel')}</button>
      <button class="btn danger-solid" onclick={confirmReset} disabled={!resetUnderstood || restarting}>
        {#if restarting}<span class="spin"><LoaderCircle size={16} /></span> {t('data.restarting')}{:else}<Trash size={16} /> {t('data.resetConfirm')}{/if}
      </button>
    {/snippet}
  </Modal>
{/if}

<style>
  .sections {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 820px;
  }
  .section-head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 18px;
  }
  .about .section-head {
    margin-bottom: 0;
  }
  .icon {
    width: 40px;
    height: 40px;
    border-radius: 13px;
    display: grid;
    place-items: center;
    flex: none;
    color: var(--primary);
    background: var(--primary-soft);
  }
  .options {
    display: grid;
    gap: 12px;
  }
  .options.two {
    grid-template-columns: repeat(2, 1fr);
  }
  .options.three {
    grid-template-columns: repeat(3, 1fr);
  }
  .option {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s, transform 0.1s;
  }
  .option:hover {
    border-color: color-mix(in srgb, var(--primary) 45%, var(--border));
  }
  .option:active {
    transform: scale(0.99);
  }
  .option.selected {
    border-color: var(--primary);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 15%, transparent);
  }
  .badge {
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 14px;
    display: grid;
    place-items: center;
    font-size: 20px;
    font-weight: 800;
    color: var(--primary);
    background: var(--primary-soft);
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .name {
    font-size: 16px;
    font-weight: 700;
  }
  .check-mark {
    width: 22px;
    height: 22px;
    flex: none;
    border-radius: 50%;
    display: grid;
    place-items: center;
    border: 1.5px solid var(--border);
    color: var(--primary-text);
    margin-left: auto;
  }
  .selected .check-mark {
    background: var(--primary);
    border-color: var(--primary);
  }
  .option.theme {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
    padding: 12px;
  }
  .theme-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 650;
    padding: 0 4px;
  }
  /* Tiny window mockups */
  .preview {
    display: flex;
    height: 84px;
    border-radius: 12px;
    overflow: hidden;
    border: 1px solid var(--border);
  }
  .pv-side {
    width: 28%;
  }
  .pv-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
  }
  .pv-line {
    height: 7px;
    width: 70%;
    border-radius: 4px;
  }
  .pv-line.short {
    width: 45%;
  }
  .pv-card {
    flex: 1;
    border-radius: 7px;
  }
  .preview.light {
    background: #f4f5fa;
  }
  .light .pv-side {
    background: #fbfbfe;
    border-right: 1px solid #e5e7ef;
  }
  .light .pv-line {
    background: #d9dbe6;
  }
  .light .pv-card {
    background: #fff;
    border: 1px solid #e5e7ef;
  }
  .preview.dark {
    background: #131419;
  }
  .dark .pv-side {
    background: #17181f;
    border-right: 1px solid #2b2e39;
  }
  .dark .pv-line {
    background: #343846;
  }
  .dark .pv-card {
    background: #1c1e26;
    border: 1px solid #2b2e39;
  }
  .preview.system {
    background: linear-gradient(135deg, #f4f5fa 50%, #131419 50%);
  }
  .system .pv-side {
    background: linear-gradient(135deg, #fbfbfe 50%, #17181f 50%);
  }
  .system .pv-line {
    background: #8b8fa0;
  }
  .system .pv-card {
    background: linear-gradient(135deg, #fff 50%, #1c1e26 50%);
  }
  .preview .pv-line:first-child {
    background: var(--primary);
    opacity: 0.8;
  }
  .cal-opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px 16px;
    padding: 12px 0;
    border-top: 1px solid var(--border);
    cursor: pointer;
  }
  .cal-opt > span {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1 1 260px;
  }
  .panel-widgets {
    cursor: default;
  }
  .widget-picks {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
    flex: 1 1 100%;
  }
  .widget-pick {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13.5px;
    cursor: pointer;
  }
  .cal-opt-title {
    font-weight: 650;
  }
  .accents {
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  }
  .option.accent {
    padding: 12px;
    gap: 10px;
  }
  .swatch {
    position: relative;
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: 11px;
  }
  .swatch-soft {
    position: absolute;
    right: -4px;
    bottom: -4px;
    width: 16px;
    height: 16px;
    border-radius: 6px;
    border: 2px solid var(--surface);
  }
  .option.accent .name {
    font-size: 14px;
    flex: 1;
    min-width: 0;
  }
  .custom-badge {
    display: grid;
    place-items: center;
    background: var(--surface);
    color: var(--text);
  }
  .custom-editor {
    display: flex;
    flex-direction: column;
    gap: 14px;
    margin-top: 16px;
    padding: 16px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .color-field {
    display: flex;
    align-items: center;
    gap: 14px;
    cursor: pointer;
  }
  .color-field > span {
    display: flex;
    flex-direction: column;
  }
  .color-field input[type='color'] {
    flex: none;
    width: 44px;
    height: 44px;
    padding: 0;
    border: 2px solid var(--surface);
    border-radius: 12px;
    background: none;
    box-shadow: var(--shadow-sm);
    cursor: pointer;
  }
  .color-field input[type='color']::-webkit-color-swatch-wrapper {
    padding: 0;
  }
  .color-field input[type='color']::-webkit-color-swatch {
    border: none;
    border-radius: 10px;
  }
  .s-title {
    font-weight: 650;
  }
  .tint-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
  }
  .custom-preview {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    padding: 12px;
    border-radius: var(--radius-sm);
    background: var(--bg);
  }
  .chip-soft {
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--primary-soft);
    color: var(--primary);
    font-weight: 650;
    font-size: 12.5px;
  }
  .bar-sample {
    flex: 1;
    min-width: 60px;
    height: 8px;
    border-radius: 8px;
    background: linear-gradient(90deg, var(--primary), var(--accent-2));
  }
  .custom-preview .ghost {
    margin-left: auto;
  }
  .sub-label {
    margin: 4px 0 8px;
  }
  .page-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .page-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border: 1.5px solid transparent;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    cursor: grab;
    transition: opacity 0.15s, border-color 0.15s;
  }
  .page-row.off .page-icon,
  .page-row.off .page-name {
    opacity: 0.45;
  }
  .page-row.dragging {
    opacity: 0.4;
  }
  .page-row.drop-target {
    border-color: var(--primary);
  }
  .grip {
    display: grid;
    color: var(--faint);
  }
  .page-icon {
    display: grid;
    color: var(--primary);
  }
  .page-name {
    font-weight: 650;
  }
  .page-row .switch {
    margin-left: 6px;
  }
  .zooms {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(72px, 1fr));
    gap: 8px;
  }
  .zoom {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    height: 72px;
    padding: 8px 4px 10px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    font-size: 12.5px;
    font-weight: 650;
    color: var(--muted);
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .zoom:hover {
    border-color: color-mix(in srgb, var(--primary) 45%, var(--border));
  }
  .zoom.selected {
    border-color: var(--primary);
    background: var(--primary-soft);
    color: var(--primary);
  }
  .zoom-sample {
    font-weight: 750;
    line-height: 1;
    color: var(--text);
  }
  .zoom-reset {
    margin-top: 12px;
  }
  .presets {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 10px;
    margin-bottom: 18px;
  }
  .preset {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 12px 8px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    font-weight: 650;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .preset:hover {
    border-color: var(--primary);
    background: var(--primary-soft);
  }
  .screen {
    position: relative;
    width: 56px;
    height: 36px;
    border-radius: 6px;
    background: var(--surface-2);
    border: 1.5px solid var(--border);
    display: grid;
    place-items: center;
  }
  .win {
    border-radius: 3px;
    background: var(--primary);
    opacity: 0.75;
  }
  .saved {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .saved-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 14px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }
  .saved-name {
    font-weight: 650;
  }
  .save-row {
    margin-top: 12px;
    flex-wrap: wrap;
  }
  @container main (max-width: 640px) {
    .options.two,
    .options.three {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .data-actions {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(260px, 100%), 1fr));
    gap: 12px;
  }
  .data-action {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    padding: 16px;
    border: 1.5px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    text-align: left;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .data-action:hover:not(:disabled) {
    border-color: var(--primary);
    background: var(--primary-soft);
  }
  .data-action:disabled {
    cursor: progress;
  }
  .da-icon {
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 13px;
    display: grid;
    place-items: center;
    color: var(--primary);
    background: var(--primary-soft);
  }
  .da-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .strong {
    font-weight: 650;
  }
  .danger-zone {
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border));
  }
  .danger-zone .section-head {
    margin-bottom: 0;
    flex-wrap: wrap;
  }
  .danger-icon {
    color: var(--danger);
    background: var(--danger-soft);
  }
  :global(.btn.danger-solid) {
    background: var(--danger);
    border-color: transparent;
    color: #fff;
  }
  :global(.btn.danger-solid:hover:not(:disabled)) {
    background: color-mix(in srgb, var(--danger) 88%, #000);
  }
  .counts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .counts .chip {
    color: var(--text);
  }
  .caution {
    display: flex;
    gap: 12px;
    padding: 14px 16px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--warning) 14%, var(--surface));
    color: color-mix(in srgb, var(--warning) 70%, var(--text));
    line-height: 1.5;
  }
  .caution :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .caution p {
    color: var(--text);
  }
  .caution.danger {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .caution ul {
    margin: 6px 0 0;
    padding-left: 18px;
    color: var(--text);
  }
  .export-first {
    align-self: flex-start;
  }
  .understand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    font-weight: 600;
    cursor: pointer;
  }
  .spin {
    display: inline-grid;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
