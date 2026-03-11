import type { StateCreator } from "zustand";
import type { Session } from "../types/session";

export interface SessionsSlice {
  sessions: Record<number, Session>;
  setSession: (taskId: number, session: Session) => void;
  removeSession: (taskId: number) => void;
  clearSessions: () => void;
  getSessionForTask: (taskId: number) => Session | undefined;
}

export const createSessionsSlice: StateCreator<SessionsSlice, [], [], SessionsSlice> = (_set, get) => ({
  sessions: {},
  setSession: (taskId, session) => {
    _set((state) => ({ sessions: { ...state.sessions, [taskId]: session } }));
  },
  removeSession: (taskId) => {
    _set((state) => {
      const { [taskId]: _, ...remaining } = state.sessions;
      return { sessions: remaining };
    });
  },
  clearSessions: () => {
    _set({ sessions: {} });
  },
  getSessionForTask: (taskId) => get().sessions[taskId],
});
