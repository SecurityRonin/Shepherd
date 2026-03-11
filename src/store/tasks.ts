import type { StateCreator } from "zustand";
import type { Task, TaskStatus, FileDiff } from "../types/task";
import type { PermissionEvent, TaskEvent } from "../types/events";

export interface TasksSlice {
  tasks: Record<number, Task>;
  pendingPermissions: PermissionEvent[];
  setTasks: (tasks: TaskEvent[]) => void;
  upsertTask: (event: TaskEvent) => void;
  removeTask: (id: number) => void;
  setTaskDiffs: (taskId: number, diffs: FileDiff[]) => void;
  setPendingPermissions: (perms: PermissionEvent[]) => void;
  addPendingPermission: (perm: PermissionEvent) => void;
  removePendingPermission: (permId: number) => void;
  getTasksByStatus: (status: TaskStatus) => Task[];
  getTaskById: (id: number) => Task | undefined;
  getPermissionsForTask: (taskId: number) => PermissionEvent[];
}

function taskEventToTask(event: TaskEvent): Task {
  return {
    id: event.id,
    title: event.title,
    agent_id: event.agent_id,
    status: event.status as TaskStatus,
    branch: event.branch,
    repo_path: event.repo_path,
    prompt: "",
    isolation_mode: "worktree",
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };
}

export const createTasksSlice: StateCreator<TasksSlice, [], [], TasksSlice> = (set, get) => ({
  tasks: {},
  pendingPermissions: [],
  setTasks: (taskEvents) => {
    const tasks: Record<number, Task> = {};
    for (const event of taskEvents) {
      tasks[event.id] = taskEventToTask(event);
    }
    set({ tasks });
  },
  upsertTask: (event) => {
    set((state) => ({
      tasks: {
        ...state.tasks,
        [event.id]: {
          ...state.tasks[event.id],
          ...taskEventToTask(event),
          ...(state.tasks[event.id]
            ? {
                prompt: state.tasks[event.id].prompt,
                isolation_mode: state.tasks[event.id].isolation_mode,
                created_at: state.tasks[event.id].created_at,
              }
            : {}),
          updated_at: new Date().toISOString(),
        },
      },
    }));
  },
  removeTask: (id) => {
    set((state) => {
      const { [id]: _, ...remaining } = state.tasks;
      return { tasks: remaining };
    });
  },
  setTaskDiffs: (taskId, diffs) => {
    set((state) => {
      const task = state.tasks[taskId];
      if (!task) return state;
      return {
        tasks: {
          ...state.tasks,
          [taskId]: { ...task, diffs },
        },
      };
    });
  },
  setPendingPermissions: (perms) => {
    set({ pendingPermissions: perms });
  },
  addPendingPermission: (perm) => {
    set((state) => ({
      pendingPermissions: [...state.pendingPermissions, perm],
    }));
  },
  removePendingPermission: (permId) => {
    set((state) => ({
      pendingPermissions: state.pendingPermissions.filter((p) => p.id !== permId),
    }));
  },
  getTasksByStatus: (status) =>
    Object.values(get().tasks).filter((t) => t.status === status),
  getTaskById: (id) => get().tasks[id],
  getPermissionsForTask: (taskId) =>
    get().pendingPermissions.filter((p) => p.task_id === taskId),
});
