// Dev-only: lets the UI run in a normal browser (outside Tauri) with sample data.
// Loaded from main.ts only when `import.meta.env.DEV` and Tauri is absent.
// Supports `?page=todos&theme=dark` to open a specific page.
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import type { Routine, Activity, Bookmark, BookmarkFolder, CalEvent, DDay, Commission, CommissionClient, DiaryEntry, Expense, Meal, Memo, MemoGroup, Workout, PomodoroSession, Tag, Todo, TodoGroup, TrpgEntry, Book } from './api';
import { addDays, addMinutes, addMonths, dayStartTs, diffDays, eventSpan, parseYmd, timeOf, today, toDateTime } from './dates';

/** Mirrors src-tauri/src/recurrence.rs (dev preview only). */
function occurrences(first: string, r: NonNullable<CalEvent['repeat']>, from: string, to: string): string[] {
  const out: string[] = [];
  const last = r.until && r.until < to ? r.until : to;
  let n = 0;
  const emit = (d: string) => {
    if (d > last || (r.count && n >= r.count)) return false;
    n++;
    if (d >= from) out.push(d);
    return true;
  };
  const step = Math.max(1, r.interval);
  for (let k = 0; k < 5000; k++) {
    if (r.freq === 'daily') {
      if (!emit(addDays(first, k * step))) break;
    } else if (r.freq === 'weekly') {
      const days = r.weekdays.length ? [...r.weekdays].sort() : [parseYmd(first).getDay()];
      const base = addDays(addDays(first, -parseYmd(first).getDay()), k * 7 * step);
      if (base > last) break;
      if (!days.every((wd) => { const d = addDays(base, wd); return d < first || emit(d); })) break;
    } else {
      const d = addMonths(first, k * step * (r.freq === 'yearly' ? 12 : 1));
      if (d > last) break;
      if (d.slice(8) === first.slice(8) && !emit(d)) break;
    }
  }
  return out;
}

export function installMock() {
  const t = today();
  const now = Math.floor(Date.now() / 1000);
  let nextId = 100;
  const ev = (title: string, day: number, s: string, e: string, color: number, location = '', allDayEnd?: number): CalEvent => ({
    id: nextId++,
    title,
    start: toDateTime(addDays(t, day), allDayEnd !== undefined ? '00:00' : s),
    end: toDateTime(addDays(t, allDayEnd ?? day), allDayEnd !== undefined ? '00:00' : e),
    all_day: allDayEnd !== undefined,
    color,
    location,
    links: ['https://meet.example.com/nora'],
    memo: 'Bring the slides',
    repeat: null,
    exdates: [],
    occurrence: null,
    cancelled: false, tags: [], reminder: null,
  });
  const events: CalEvent[] = [
    { ...ev('Team standup', 0, '09:30', '10:00', 0x4aa8ff, 'Zoom'), tags: [1], reminder: 10 },
    { ...ev('Design review', 0, '09:45', '11:00', 0xb164e8, 'Room 3'), tags: [2, 3] },
    ev('Lunch with 지민', 0, '12:00', '13:00', 0x34c38f, 'Cafe Onion'),
    ev('Gym', 1, '18:00', '19:30', 0xff8a5c),
    ev('Conference', 2, '', '', 0x7c74ff, 'Seoul', 4),
    { ...ev('Dentist', -2, '15:00', '16:00', 0xf2668b), cancelled: true },
    { ...ev('Morning run', -3, '07:00', '07:45', 0x34c38f), repeat: { freq: 'weekly', interval: 1, weekdays: [1, 3, 5], until: null, count: null } },
  ];
  let nextTag = 4;
  let tags: Tag[] = [
    { id: 1, name: 'Meeting', color: 0x4aa8ff, category: 'Work' },
    { id: 2, name: 'Deadline', color: 0xf2668b, category: 'Work' },
    { id: 3, name: 'Health', color: 0x34c38f, category: 'Personal' },
  ];
  const ddays: DDay[] = [
    { id: 1, title: 'Final exam', date: addDays(t, 12), color: 0xf2668b, image: null, yearly: false, count_from_one: false, shape: 'normal', position: 0 },
    { id: 2, title: 'Trip to Jeju', date: addDays(t, 40), color: 0x4aa8ff, image: null, yearly: false, count_from_one: false, shape: 'normal', position: 0 },
    { id: 3, title: 'Started new job', date: addDays(t, -100), color: 0x34c38f, image: null, yearly: false, count_from_one: false, shape: 'normal', position: 0 },
  ];
  const groups: TodoGroup[] = [
    { id: 1, name: 'Work', color: 0x7c74ff },
    { id: 2, name: 'Personal', color: 0x34c38f },
  ];
  const todo = (id: number, title: string, group: number | null, parent: number | null, done = false, due: string | null = null): Todo => ({
    id, title, group_id: group, parent_id: parent, notes: id === 1 ? 'Ship before Friday' : '', done, due, due_time: id === 1 ? '18:00' : null,
    created_at: now - (id % 5) * 86_400, completed_at: done ? now - (id % 3) * 86_400 : null,
  });
  const routines: (Routine & { last_made?: string })[] = [
    { id: 9000, title: '아침에 물 한 잔 마시기', group_id: null, weekdays: [], active: true, created_at: now - 86_400 * 10 },
    { id: 9001, title: '스트레칭 10분', group_id: 2, weekdays: [1, 3, 5], active: true, created_at: now - 86_400 * 5 },
  ];
  /** Same rules as routines.rs: today's copies, once a day; unticked old copies are cleared. */
  const materialize = () => {
    const day = today();
    const wd = parseYmd(day).getDay();
    for (let i = todos.length - 1; i >= 0; i--) if (todos[i].routine_id && !todos[i].done && todos[i].due! < day) todos.splice(i, 1);
    for (const r of routines) {
      if (!r.active || (r.weekdays.length && !r.weekdays.includes(wd)) || (r.last_made ?? '') >= day) continue;
      todos.push({ ...todo(nextId++, r.title, r.group_id, null, false, day), routine_id: r.id });
      r.last_made = day;
    }
  };
  const todos: Todo[] = [
    todo(1, 'Ship v1.0', 1, null, false, addDays(t, 2)),
    todo(2, 'Fix calendar bugs', 1, 1, true),
    todo(3, 'Package the installer', 1, 1),
    todo(4, 'Write release notes', 1, null, true),
    todo(5, 'Buy groceries', 2, null, false, t),
    todo(6, 'Call mom', null, null, true),
    todo(7, 'Renew passport', 2, null, false, addDays(t, -1)),
  ];
  const memos: Memo[] = [
    { id: 1, title: 'Weekly sync', body: '# Weekly sync\n\n- [x] Review roadmap\n- [ ] Plan **Q4** goals\n\n> Keep it simple.\n\n```rust\nfn main() {}\n```\n\n한국어도 잘 보입니다.', created_at: now, updated_at: now, group_id: 1 },
    { id: 2, title: 'Ideas', body: 'A calm, friendly scheduler.', created_at: now - 3600, updated_at: now - 3600, group_id: null },
  ];
  const memoGroups: MemoGroup[] = [{ id: 1, name: 'Work', color: 0x7c74ff }, { id: 2, name: 'Diary', color: 0x34c38f }];
  const sessions: PomodoroSession[] = [];
  const activity: Activity[] = [];
  const apps: [string, string][] = [['Code', 'main.rs — NoraSchedule'], ['Google Chrome', 'Svelte docs'], ['Slack', '#general'], ['Figma', 'Nora UI']];
  for (let d = 0; d < 30; d++) {
    let ts = dayStartTs(addDays(t, -d)) + 9 * 3600;
    apps.forEach(([app, title], i) => {
      const len = 1800 + ((d * 7 + i * 13) % 5) * 1200;
      if (ts + len < now) activity.push({ app, title, start: ts, end: ts + len });
      ts += len + 600;
    });
    const s = dayStartTs(addDays(t, -d)) + 14 * 3600;
    if (s + 1500 < now && d % 3 !== 2) sessions.push({ id: 7000 + d, started_at: s, ended_at: s + 1500, label: 'Ship v1.0', focused_secs: 1500, completed: true });
  }
  const folders: BookmarkFolder[] = [
    { id: 1, name: 'Work', parent_id: null, color: 0x7c74ff },
    { id: 2, name: 'Specs', parent_id: 1, color: 0x4aa8ff },
    { id: 3, name: 'Reading', parent_id: null, color: 0x34c38f },
  ];
  const link = (id: number, folder: number | null, title: string, url: string, kind: string, note = ''): Bookmark =>
    ({ id, folder_id: folder, title, url, kind, note, created_at: now - id * 3600 });
  const bookmarks: Bookmark[] = [
    link(1, 1, 'Q4 roadmap', 'https://docs.google.com/document/d/abc', 'gdoc', 'Team goals'),
    link(2, 1, 'Budget 2026', 'https://docs.google.com/spreadsheets/d/xyz', 'gsheet'),
    link(3, 1, 'Team drive', 'https://drive.google.com/drive/folders/1', 'gdrive'),
    link(4, 2, 'API design notes', 'https://www.notion.so/api-notes', 'notes'),
    link(5, 3, 'How to write clearly', 'https://medium.com/@writer/clear-writing', 'blog'),
    link(6, 3, 'Svelte 5 runes explained', 'https://www.youtube.com/watch?v=abc', 'video'),
    link(7, null, 'NoraSchedule repo', 'https://github.com/nora/schedule', 'code'),
  ];
  const expenses: Expense[] = [];
  const cats = ['food', 'cafe', 'transport', 'shopping', 'food', 'cafe', 'bills', 'fun'];
  for (let d = 0; d < 40; d++) {
    const day = addDays(t, -d);
    for (let k = 0; k < 1 + (d % 3); k++) {
      const c = cats[(d * 3 + k) % cats.length];
      const base = { food: 9000, cafe: 4800, transport: 1500, shopping: 32000, bills: 55000, fun: 15000 }[c] ?? 10000;
      expenses.push({ id: 1000 + d * 10 + k, amount: base + ((d * 7 + k * 13) % 9) * 500, category: c, date: day, note: k === 0 && c === 'food' ? 'Lunch' : '' });
    }
  }
  const diary: DiaryEntry[] = [
    { date: addDays(t, -1), mood: 'great', layout: 'classic', paper: 'lined', line: '바다를 보고 왔다', body: '<p>오랜만에 <mark data-color="#fde68a" style="background-color: #fde68a">바다</mark>에 다녀왔다. 바람이 시원했다.</p>', photo: null, stickers: [], updated_at: now - 86_400 },
    { date: addDays(t, -2), mood: 'tired', layout: 'simple', paper: 'grid', line: '마감 주간', body: '<p>외주 마감이 겹쳐서 하루 종일 작업했다.</p>', photo: null, stickers: [], updated_at: now - 86_400 * 2 },
    { date: addDays(t, -4), mood: 'good', layout: 'classic', paper: 'dots', line: '', body: '<p>새 노트를 샀다.</p>', photo: null, stickers: [], updated_at: now - 86_400 * 4 },
    { date: addDays(t, -40), mood: 'love', layout: 'photo', paper: 'plain', line: '생일', body: '<p>케이크!</p>', photo: null, stickers: [], updated_at: now - 86_400 * 40 },
    { date: addDays(t, -120), mood: 'sad', layout: 'classic', paper: 'lined', line: '', body: '<p>비가 왔다.</p>', photo: null, stickers: [], updated_at: now - 86_400 * 120 },
  ];
  const clients: CommissionClient[] = [
    { id: 8000, name: '하루', contact: '@haru_draws', memo: '단골, 수정 1회 무료' },
    { id: 8001, name: 'Mido', contact: 'mido@example.com', memo: '' },
  ];
  const commission = (id: number, title: string, client_id: number | null, kind: string, status: Commission['status'], price: number, extra: Partial<Commission> = {}): Commission => ({
    id, title, client_id, kind, status, price, payments: [], deadline: null, revisions: 0, links: [], image: null, memo: '',
    created_at: now - 86_400 * (9000 - id), delivered: null, ...extra,
  });
  const commissions: Commission[] = [
    commission(8100, '하루님 자캐 흉상', 8000, '흉상', 'working', 60000, { deadline: addDays(t, 3), payments: [{ amount: 30000, date: addDays(t, -6) }], revisions: 1 }),
    commission(8101, 'Mido SD 2인', 8001, 'SD', 'review', 45000, { deadline: addDays(t, -1), payments: [{ amount: 45000, date: addDays(t, -10) }], revisions: 2 }),
    commission(8102, '전신 일러스트 문의', null, '전신', 'inquiry', 0, { memo: '배경 포함 견적 요청' }),
    commission(8103, '하루님 로고', 8000, '로고', 'queued', 80000, { deadline: addDays(t, 14) }),
    commission(8104, '두상 3인', 8001, '두상', 'done', 36000, { payments: [{ amount: 36000, date: addDays(t, -2) }], delivered: addDays(t, -2) }),
  ];
  const meals: Meal[] = [];
  const workouts: Workout[] = [];
  const foods: [Meal['slot'], string, number][] = [
    ['breakfast', 'Greek yogurt & granola', 320], ['breakfast', '아메리카노', 10], ['lunch', '김밥', 450],
    ['lunch', 'Chicken salad', 420], ['dinner', '된장찌개 & 밥', 620], ['snack', 'Banana', 105], ['dinner', 'Salmon bowl', 680],
  ];
  for (let d = -6; d <= 1; d++) {
    const day = addDays(t, d);
    foods.forEach(([slot, name, kcal], i) => {
      if ((i + d) % 3 === 0 && d !== 0) return;
      meals.push({ id: 5000 + (d + 6) * 10 + i, date: day, slot, name, kcal, eaten: d < 0 || (d === 0 && slot !== 'dinner') });
    });
    if (d <= 0 && d % 2 === 0) workouts.push({ id: 6000 + d + 6, date: day, kind: d === 0 ? 'running' : 'walking', minutes: d === 0 ? 30 : 45, kcal: d === 0 ? 294 : 158, note: '' });
  }
  const trpg: TrpgEntry[] = ([
    { id: 2000, kind: 'rulebook', title: 'Call of Cthulhu 7th Edition', writer: 'Chaosium', system: 'CoC 7th', links: [], image: null, date: null, role: '', pair: '', folder: '', memo: '', created_at: now - 86_400 * 90 },
    { id: 2001, kind: 'scenario_book', title: 'Doors to Darkness', writer: 'Chaosium', system: 'CoC 7th', links: [], image: null, date: null, role: '', pair: '', folder: '', memo: '5 starter scenarios', created_at: now - 86_400 * 60 },
    { id: 2002, kind: 'played', title: 'COSMOS', writer: '노라', system: 'CoC 7th', links: ['https://example.com/cosmos', 'https://example.com/cosmos-log'], image: null, date: '2026-03-03', role: 'pl,HO1', pair: '', folder: 'CoC 타이만', memo: '', created_at: now - 86_400 * 200 },
    { id: 2003, kind: 'played', title: 'The Haunting', writer: 'Sandy Petersen', system: 'CoC 7th', links: [], image: null, date: '2025-11-20', role: 'gm', pair: '', folder: '', memo: '', created_at: now - 86_400 * 300 },
    { id: 2005, kind: 'played', title: '가면무도회', writer: '하루', system: 'CoC 7th', links: [], image: null, date: '2026-05-10', role: 'gm', pair: '', folder: '', memo: '', created_at: now - 86_400 * 150 },
    { id: 2006, kind: 'played', title: '하늘의 노래', writer: '미도', system: 'Insane', links: ['https://example.com/sky'], image: null, date: '2025-07-01', role: 'pl', pair: '하루 & 미도', folder: '', memo: '', created_at: now - 86_400 * 400 },
    { id: 2007, kind: 'rulebook', title: '인세인', writer: '모험기획국', system: 'Insane', links: [], image: null, date: null, role: '', pair: '', folder: '', memo: '', created_at: now - 86_400 * 80 },
    { id: 2008, kind: 'rulebook', title: 'Delta Green', writer: 'Arc Dream', system: 'Delta Green', links: [], image: null, date: null, role: '', pair: '', folder: '', memo: '', created_at: now - 86_400 * 70 },
    { id: 2009, kind: 'wishlist', title: '달빛 아래', writer: '노라', system: 'CoC 7th', links: [], image: null, date: null, role: '', pair: '', folder: '', memo: '', created_at: now - 86_400 * 3 },
    { id: 2004, kind: 'wishlist', title: 'Masks of Nyarlathotep', writer: 'Larry DiTillio', system: 'CoC 7th', links: ['https://example.com/masks'], image: null, date: null, role: '', pair: '', folder: '', memo: 'Long campaign', created_at: now - 86_400 * 10 },
  ] as Omit<TrpgEntry, 'position' | 'play_count'>[]).map((e) => ({ ...e, position: 0, play_count: e.id === 2002 ? 2 : 0 }));
  const book = (id: number, title: string, author: string, status: Book['status'], extra: Partial<Book> = {}): Book => ({
    id, title, author, publisher: '', status, image: null, total_pages: 0, current_page: 0, rating: 0,
    started: null, finished: null, review: '', created_at: now - 86_400 * (100 - id + 3000), ...extra,
  });
  const books: Book[] = [
    book(3000, '소년이 온다', '한강', 'reading', { total_pages: 216, current_page: 80, started: addDays(t, -7) }),
    book(3001, 'Project Hail Mary', 'Andy Weir', 'reading', { total_pages: 496, current_page: 410, started: addDays(t, -20) }),
    book(3002, '아몬드', '손원평', 'read', { total_pages: 264, current_page: 264, rating: 9, started: '2026-08-01', finished: '2026-08-12', review: '감정을 배우는 이야기.\n\n**곤이**가 오래 남는다.' }),
    book(3003, '불편한 편의점', '김호연', 'read', { total_pages: 268, current_page: 268, rating: 7, finished: '2026-03-02' }),
    book(3004, 'The Midnight Library', 'Matt Haig', 'read', { rating: 8, finished: '2025-11-20' }),
    book(3005, '데미안', '헤르만 헤세', 'want'),
    book(3006, '어떤 물질의 사랑', '천선란', 'want'),
    book(3007, 'Klara and the Sun', 'Kazuo Ishiguro', 'want'),
  ];
  const settings: Record<string, string> = { 'expense.currency': 'KRW', 'expense.budget': '1200000' };
  const previewLocale = new URLSearchParams(location.search).get('locale');
  if (previewLocale) settings['ui.locale'] = previewLocale;
  let tracker = { paused: false, idle_threshold_secs: 300, tracked_apps: ['Code', 'Google Chrome'] };

  // The window code (overlay panel, sync) asks which window it is running in.
  mockWindows('main');
  mockIPC((cmd, a: any) => {
    switch (cmd) {
      case 'get_setting': return settings[a.key] ?? null;
      case 'set_setting': settings[a.key] = a.value; return null;
      case 'events_between': {
        const out: CalEvent[] = [];
        for (const e of events) {
          if (!e.repeat) { out.push(e); continue; }
          const [first, lastDay] = eventSpan(e);
          const length = (new Date(e.end).getTime() - new Date(e.start).getTime()) / 60000;
          for (const d of occurrences(first, e.repeat, addDays(a.from, -diffDays(lastDay, first)), a.to)) {
            if (e.exdates.includes(d)) continue;
            const start = toDateTime(d, timeOf(e.start));
            out.push({ ...e, start, end: addMinutes(start, length), occurrence: d });
          }
        }
        return out.filter((e) => { const [x, y] = eventSpan(e); return x <= a.to && y >= a.from; }).sort((x, y) => x.start.localeCompare(y.start));
      }
      case 'save_event': {
        const ev: CalEvent = a.event;
        const stored = events.find((e) => e.id === ev.id);
        if (!stored) { events.push({ ...ev, id: nextId, exdates: [], occurrence: null }); return nextId++; }
        if (stored.repeat && ev.occurrence) {
          if (a.scope === 'one') {
            stored.exdates = [...stored.exdates, ev.occurrence];
            events.push({ ...ev, id: nextId, repeat: null, exdates: [], occurrence: null });
            return nextId++;
          }
          const shift = (new Date(ev.start).getTime() - new Date(toDateTime(ev.occurrence, timeOf(stored.start))).getTime()) / 60000;
          const start = addMinutes(stored.start, shift);
          const length = (new Date(ev.end).getTime() - new Date(ev.start).getTime()) / 60000;
          Object.assign(stored, { ...ev, start, end: addMinutes(start, length), occurrence: null, exdates: stored.exdates });
          return ev.id;
        }
        Object.assign(stored, { ...ev, occurrence: null });
        return ev.id;
      }
      case 'delete_event': {
        const i = events.findIndex((e) => e.id === a.id);
        if (i < 0) return null;
        const stored = { ...events[i] };
        if (a.scope === 'one' && a.occurrence && stored.repeat) events[i].exdates = [...events[i].exdates, a.occurrence];
        else events.splice(i, 1);
        return stored;
      }
      case 'restore_occurrence': {
        const e = events.find((x) => x.id === a.id);
        if (e) e.exdates = e.exdates.filter((d) => d !== a.day);
        return null;
      }
      case 'tags': return [...tags];
      case 'save_tag': { if (!a.tag.id) { tags.push({ ...a.tag, id: nextTag }); return nextTag++; } tags = tags.map((x) => (x.id === a.tag.id ? a.tag : x)); return a.tag.id; }
      case 'delete_tag': tags = tags.filter((x) => x.id !== a.id); for (const e of events) e.tags = e.tags.filter((id) => id !== a.id); return null;
      case 'ddays': return [...ddays].sort((x, y) => x.date.localeCompare(y.date));
      case 'save_dday': if (a.dday.id) Object.assign(ddays.find((d) => d.id === a.dday.id)!, a.dday); else ddays.push({ ...a.dday, id: nextId++, position: Math.max(0, ...ddays.map((d) => d.position)) + 1 }); return null;
      case 'reorder_ddays': (a.ids as number[]).forEach((id, i) => { const d = ddays.find((x) => x.id === id); if (d) d.position = i; }); return null;
      case 'delete_dday': ddays.splice(ddays.findIndex((d) => d.id === a.id), 1); return null;
      case 'import_image': {
        const w = window as Window & { __noraPreviewImages?: Map<string, string> };
        w.__noraPreviewImages ??= new Map();
        const name = `preview-${nextId++}.jpg`;
        w.__noraPreviewImages.set(name, URL.createObjectURL(new Blob([a as BlobPart])));
        return name;
      }
      case 'remove_unused_images': return null;
      case 'todo_groups': return groups;
      case 'add_todo_group': groups.push({ id: nextId, name: a.name, color: a.color }); return nextId++;
      case 'update_todo_group': Object.assign(groups.find((g) => g.id === a.group.id)!, a.group); return null;
      case 'delete_todo_group': groups.splice(groups.findIndex((g) => g.id === a.id), 1); todos.forEach((x) => x.group_id === a.id && (x.group_id = null)); return null;
      case 'todos': materialize(); return todos;
      case 'routines': return routines;
      case 'save_routine': {
        const r = a.routine as Routine;
        if (r.id) { Object.assign(routines.find((x) => x.id === r.id)!, r); todos.forEach((x) => x.routine_id === r.id && !x.done && Object.assign(x, { title: r.title, group_id: r.group_id })); return r.id; }
        routines.push({ ...r, id: nextId, created_at: now }); return nextId++;
      }
      case 'delete_routine': {
        for (let i = todos.length - 1; i >= 0; i--) if (todos[i].routine_id === a.id && !todos[i].done) todos.splice(i, 1);
        routines.splice(routines.findIndex((x) => x.id === a.id), 1); return null;
      }
      case 'add_todo': todos.push({ ...todo(nextId, a.title, a.groupId, a.parentId, false, a.due), due_time: a.due ? a.dueTime : null }); return nextId++;
      case 'update_todo': Object.assign(todos.find((x) => x.id === a.todo.id)!, { title: a.todo.title, notes: a.todo.notes, due: a.todo.due, due_time: a.todo.due_time, group_id: a.todo.group_id }); return null;
      case 'set_todo_done': {
        const mark = (id: number) => { const x = todos.find((y) => y.id === id)!; x.done = a.done; x.completed_at = a.done ? now : null; if (a.done) todos.filter((c) => c.parent_id === id).forEach((c) => mark(c.id)); };
        mark(a.id); return null;
      }
      case 'delete_todo': { const drop = (id: number) => { todos.filter((c) => c.parent_id === id).forEach((c) => drop(c.id)); todos.splice(todos.findIndex((x) => x.id === id), 1); }; drop(a.id); return null; }
      case 'memos': return [...memos].sort((x, y) => y.updated_at - x.updated_at);
      case 'create_memo': memos.push({ id: nextId, title: a.title, body: '', created_at: now, updated_at: Date.now() / 1000, group_id: a.groupId ?? null }); return nextId++;
      case 'set_memo_group': memos.find((m) => m.id === a.id)!.group_id = a.groupId; return null;
      case 'memo_groups': return memoGroups;
      case 'save_memo_group': {
        if (a.group.id) { Object.assign(memoGroups.find((g) => g.id === a.group.id)!, a.group); return a.group.id; }
        memoGroups.push({ ...a.group, id: nextId }); return nextId++;
      }
      case 'delete_memo_group': {
        memos.forEach((m) => m.group_id === a.id && (m.group_id = null));
        memoGroups.splice(memoGroups.findIndex((g) => g.id === a.id), 1); return null;
      }
      case 'update_memo': Object.assign(memos.find((m) => m.id === a.id)!, { title: a.title, body: a.body, updated_at: Date.now() / 1000 }); return null;
      case 'delete_memo': memos.splice(memos.findIndex((m) => m.id === a.id), 1); return null;
      case 'save_pomodoro_session': {
        const row = { id: a.id ?? nextId++, started_at: a.startedAt, ended_at: a.endedAt, label: a.label, focused_secs: a.focusedSecs, completed: a.completed };
        const i = sessions.findIndex((s) => s.id === row.id);
        if (i < 0) sessions.push(row); else sessions[i] = row;
        return row.id;
      }
      case 'pomodoro_sessions': return sessions.filter((s) => s.ended_at > a.fromTs && s.started_at < a.toTs);
      case 'activity_summary': {
        const days: string[] = a.days;
        const per_day = days.map((d) => { const s = dayStartTs(d), e = s + 86_400; return activity.reduce((sum, x) => sum + Math.max(0, Math.min(x.end, e) - Math.max(x.start, s)), 0); });
        const from = dayStartTs(days[0]), to = dayStartTs(days[days.length - 1]) + 86_400;
        const inRange = activity.filter((x) => x.end > from && x.start < to);
        const appMap = new Map<string, number>(), titleMap = new Map<string, { app: string; title: string; secs: number }>();
        for (const x of inRange) {
          appMap.set(x.app, (appMap.get(x.app) ?? 0) + x.end - x.start);
          const k = x.app + '\u0000' + x.title; const v = titleMap.get(k) ?? { app: x.app, title: x.title, secs: 0 }; v.secs += x.end - x.start; titleMap.set(k, v);
        }
        return {
          total: per_day.reduce((s, x) => s + x, 0), per_day,
          apps: [...appMap].map(([app, secs]) => ({ app, secs })).sort((x, y) => y.secs - x.secs),
          titles: [...titleMap.values()].sort((x, y) => y.secs - x.secs).slice(0, a.titleLimit),
        };
      }
      case 'activity_timeline': { const s = dayStartTs(a.day); return activity.filter((x) => x.end > s && x.start < s + 86_400); }
      case 'tracker_status': return tracker.paused ? { state: 'paused', app: '', title: '', segment_secs: 0, recent_apps: [] } : { state: 'tracking', app: 'Code', title: 'App.svelte — NoraSchedule', segment_secs: 754, recent_apps: ['Code', 'Google Chrome', 'Slack', 'Spotify'] };
      case 'tracker_settings': return tracker;
      case 'set_tracker_settings': tracker = a.settings; return null;
      case 'bookmark_folders': return folders;
      case 'save_bookmark_folder': {
        if (a.folder.id) { Object.assign(folders.find((f) => f.id === a.folder.id)!, a.folder); return a.folder.id; }
        folders.push({ ...a.folder, id: nextId }); return nextId++;
      }
      case 'delete_bookmark_folder': {
        const f = folders.find((x) => x.id === a.id);
        if (!f) return null;
        bookmarks.forEach((b) => b.folder_id === a.id && (b.folder_id = f.parent_id));
        folders.forEach((x) => x.parent_id === a.id && (x.parent_id = f.parent_id));
        folders.splice(folders.indexOf(f), 1); return null;
      }
      case 'bookmarks': return [...bookmarks].sort((x, y) => y.created_at - x.created_at);
      case 'save_bookmark': {
        if (a.bookmark.id) { Object.assign(bookmarks.find((b) => b.id === a.bookmark.id)!, a.bookmark); return a.bookmark.id; }
        bookmarks.push({ ...a.bookmark, id: nextId, created_at: a.bookmark.created_at || Math.floor(Date.now() / 1000) }); return nextId++;
      }
      case 'reorder_bookmarks': bookmarks.sort((x, y) => a.ids.indexOf(x.id) - a.ids.indexOf(y.id)); return null;
      case 'delete_bookmark': { const i = bookmarks.findIndex((b) => b.id === a.id); return i < 0 ? null : bookmarks.splice(i, 1)[0]; }
      case 'expenses_between': return expenses.filter((e) => e.date >= a.from && e.date <= a.to).sort((x, y) => y.date.localeCompare(x.date) || y.id - x.id);
      case 'save_expense': {
        if (a.expense.id) { Object.assign(expenses.find((e) => e.id === a.expense.id)!, a.expense); return a.expense.id; }
        expenses.push({ ...a.expense, id: nextId }); return nextId++;
      }
      case 'delete_expense': { const i = expenses.findIndex((e) => e.id === a.id); return i < 0 ? null : expenses.splice(i, 1)[0]; }
      case 'diary_between': return diary.filter((e) => e.date >= a.from && e.date <= a.to).sort((x, y) => x.date.localeCompare(y.date));
      case 'save_diary_entry': {
        const e = a.entry as DiaryEntry;
        const blank = !e.mood && !e.line.trim() && !e.body.replace(/<[^>]*>/g, '').trim() && !e.photo && !e.stickers.length;
        const i = diary.findIndex((x) => x.date === e.date);
        if (blank) { if (i >= 0) diary.splice(i, 1); return null; }
        if (i >= 0) diary[i] = { ...e, updated_at: Math.floor(Date.now() / 1000) }; else diary.push({ ...e, updated_at: Math.floor(Date.now() / 1000) });
        return null;
      }
      case 'delete_diary_entry': { const i = diary.findIndex((x) => x.date === a.date); return i < 0 ? null : diary.splice(i, 1)[0]; }
      case 'commissions': return [...commissions].sort((x, y) => y.created_at - x.created_at);
      case 'save_commission': {
        const c = { ...a.commission, delivered: a.commission.status === 'done' ? a.commission.delivered ?? today() : null };
        if (c.id) { Object.assign(commissions.find((x) => x.id === c.id)!, c); return c.id; }
        commissions.push({ ...c, id: nextId, created_at: c.created_at || Math.floor(Date.now() / 1000) }); return nextId++;
      }
      case 'delete_commission': { const i = commissions.findIndex((x) => x.id === a.id); return i < 0 ? null : commissions.splice(i, 1)[0]; }
      case 'commission_clients': return [...clients].sort((x, y) => x.name.localeCompare(y.name));
      case 'save_commission_client': {
        if (a.client.id) { Object.assign(clients.find((x) => x.id === a.client.id)!, a.client); return a.client.id; }
        clients.push({ ...a.client, id: nextId }); return nextId++;
      }
      case 'delete_commission_client': {
        commissions.forEach((x) => x.client_id === a.id && (x.client_id = null));
        clients.splice(clients.findIndex((x) => x.id === a.id), 1); return null;
      }
      case 'meals_between': return meals.filter((m) => m.date >= a.from && m.date <= a.to).sort((x, y) => x.date.localeCompare(y.date) || x.id - y.id);
      case 'save_meal': {
        if (a.meal.id) { Object.assign(meals.find((m) => m.id === a.meal.id)!, a.meal); return a.meal.id; }
        meals.push({ ...a.meal, id: nextId }); return nextId++;
      }
      case 'delete_meal': { const i = meals.findIndex((m) => m.id === a.id); return i < 0 ? null : meals.splice(i, 1)[0]; }
      case 'workouts_between': return workouts.filter((w) => w.date >= a.from && w.date <= a.to).sort((x, y) => x.date.localeCompare(y.date) || x.id - y.id);
      case 'save_workout': {
        if (a.workout.id) { Object.assign(workouts.find((w) => w.id === a.workout.id)!, a.workout); return a.workout.id; }
        workouts.push({ ...a.workout, id: nextId }); return nextId++;
      }
      case 'delete_workout': { const i = workouts.findIndex((w) => w.id === a.id); return i < 0 ? null : workouts.splice(i, 1)[0]; }
      case 'trpg_entries': return [...trpg].sort((x, y) => x.kind.localeCompare(y.kind) || (y.date ?? '').localeCompare(x.date ?? '') || y.created_at - x.created_at);
      case 'save_trpg_entry': {
        if (a.entry.id) { Object.assign(trpg.find((e) => e.id === a.entry.id)!, a.entry); return a.entry.id; }
        const last = Math.max(0, ...trpg.filter((e) => e.kind === a.entry.kind).map((e) => e.position));
        trpg.push({ ...a.entry, id: nextId, created_at: a.entry.created_at || now, position: last + 1 }); return nextId++;
      }
      case 'reorder_trpg_entries': (a.ids as number[]).forEach((id, i) => { const e = trpg.find((x) => x.id === id); if (e) e.position = i; }); return null;
      case 'delete_trpg_entry': { const i = trpg.findIndex((e) => e.id === a.id); return i < 0 ? null : trpg.splice(i, 1)[0]; }
      case 'books': return [...books].sort((x, y) => y.created_at - x.created_at);
      case 'save_book': {
        if (a.book.id) { Object.assign(books.find((b) => b.id === a.book.id)!, a.book); return a.book.id; }
        books.push({ ...a.book, id: nextId, created_at: a.book.created_at || Math.floor(Date.now() / 1000) }); return nextId++;
      }
      case 'delete_book': { const i = books.findIndex((b) => b.id === a.id); return i < 0 ? null : books.splice(i, 1)[0]; }
      case 'plugin:app|version': return '0.5.0';
      case 'latest_release': return { version: '0.6.0', url: 'https://github.com/ImgyeongLee/NoraSchedule/releases/tag/v0.6.0' };
      case 'plugin:dialog|save': return '/Users/you/Documents/Nora-backup.nora';
      case 'plugin:dialog|open': return '/Users/you/Documents/Nora-backup.nora';
      case 'export_data':
      case 'inspect_backup':
        return { format: 1, app_version: '0.1.0', exported_at: now - 86_400 * 3,
          counts: { events: events.length, todos: todos.length, memos: memos.length, ddays: ddays.length, bookmarks: bookmarks.length, expenses: expenses.length, trpg: trpg.length, books: books.length, meals: meals.length, workouts: workouts.length, commissions: commissions.length, diary: diary.length, images: 2 } };
      case 'import_data':
      case 'reset_all_data':
        return null;
      default: return null;
    }
  });
}
