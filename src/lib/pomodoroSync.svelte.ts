// The overlay panel shows the Pomodoro timer of the main window. The main window runs the one
// real timer (it records sessions and rings); it sends its state whenever that changes, and the
// panel sends back the buttons pressed there.
import { emit, listen } from '@tauri-apps/api/event';
import { pomodoro, type PomodoroCommand, type PomodoroSnapshot } from './pomodoro.svelte';

const STATE = 'nora://pomodoro/state';
const COMMAND = 'nora://pomodoro/command';
const ASK = 'nora://pomodoro/ask';

const inTauri = () => '__TAURI_INTERNALS__' in window;

/** In the main window: share the timer and carry out the panel's buttons. */
export function sharePomodoro() {
  if (!inTauri()) return;
  const send = (s: PomodoroSnapshot) => emit(STATE, s).catch(() => {});
  $effect.root(() => {
    // Reads every field of the snapshot, so it runs again whenever one changes (not every tick).
    $effect(() => {
      send(pomodoro.snapshot());
    });
  });
  listen<PomodoroCommand>(COMMAND, (e) => pomodoro.run(e.payload));
  listen(ASK, () => send(pomodoro.snapshot()));
}

/** In the overlay panel: show the main window's timer and pass button presses to it. */
export function mirrorPomodoro() {
  if (!inTauri()) return;
  pomodoro.mirrorOf((command) => emit(COMMAND, command).catch(() => {}));
  listen<PomodoroSnapshot>(STATE, (e) => pomodoro.applySnapshot(e.payload)).then(() => emit(ASK, {}).catch(() => {}));
}
