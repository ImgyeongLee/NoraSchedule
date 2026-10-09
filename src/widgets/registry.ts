// Which Svelte component draws each Overview widget. Shared by the Overview page and
// the overlay panel, so both show exactly the same widgets.
import type { Component } from 'svelte';
import type { WidgetId } from '../lib/home.svelte';
import AgendaWidget from './AgendaWidget.svelte';
import BookmarksWidget from './BookmarksWidget.svelte';
import CalendarWidget from './CalendarWidget.svelte';
import DDayCardWidget from './DDayCardWidget.svelte';
import DDaysWidget from './DDaysWidget.svelte';
import ExpensesWidget from './ExpensesWidget.svelte';
import GreetingWidget from './GreetingWidget.svelte';
import HealthWidget from './HealthWidget.svelte';
import ImageCardWidget from './ImageCardWidget.svelte';
import MemosWidget from './MemosWidget.svelte';
import PomodoroWidget from './PomodoroWidget.svelte';
import ProgressWidget from './ProgressWidget.svelte';
import TodosWidget from './TodosWidget.svelte';
import WeekChartWidget from './WeekChartWidget.svelte';
import WorkingWidget from './WorkingWidget.svelte';

export const WIDGET_COMPONENTS: Record<WidgetId, Component<any>> = {
  dday: DDayCardWidget,
  bookmarks: BookmarksWidget,
  expenses: ExpensesWidget,
  health: HealthWidget,
  greeting: GreetingWidget,
  agenda: AgendaWidget,
  calendar: CalendarWidget,
  todos: TodosWidget,
  ddays: DDaysWidget,
  pomodoro: PomodoroWidget,
  working: WorkingWidget,
  weekChart: WeekChartWidget,
  progress: ProgressWidget,
  memos: MemosWidget,
  image: ImageCardWidget,
};
