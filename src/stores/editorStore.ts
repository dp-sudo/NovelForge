import { create } from "zustand";

export type SaveStatus = "saved" | "saving" | "unsaved" | "autosaving" | "error";

export type AiStreamStatus = "idle" | "streaming" | "completed" | "error";

export interface EditorState {
  activeChapterId: string | null;
  activeChapterTitle: string;
  content: string;
  saveStatus: SaveStatus;
  lastSavedAt: string | null;
  isDirty: boolean;
  wordCount: number;
  aiStreamStatus: AiStreamStatus;
  aiPreviewContent: string;
  aiRequestId: string | null;
  aiTaskType: string;
  setActiveChapter: (id: string, title: string) => void;
  setContent: (content: string) => void;
  setSaveStatus: (status: SaveStatus) => void;
  setLastSavedAt: (time: string) => void;
  setIsDirty: (dirty: boolean) => void;
  setWordCount: (count: number) => void;
  setAiStreamStatus: (status: AiStreamStatus) => void;
  setAiPreviewContent: (content: string) => void;
  appendAiPreviewContent: (delta: string) => void;
  setAiRequestId: (id: string | null) => void;
  setAiTaskType: (taskType: string) => void;
  resetAiPreview: () => void;
  reset: () => void;
}

const initialState = {
  activeChapterId: null,
  activeChapterTitle: "",
  content: "",
  saveStatus: "saved" as SaveStatus,
  lastSavedAt: null,
  isDirty: false,
  wordCount: 0,
  aiStreamStatus: "idle" as AiStreamStatus,
  aiPreviewContent: "",
  aiRequestId: null,
  aiTaskType: ""
};

export const useEditorStore = create<EditorState>((set) => ({
  ...initialState,
  setActiveChapter: (id, title) =>
    set({ activeChapterId: id, activeChapterTitle: title }),
  setContent: (content) => set({ content }),
  setSaveStatus: (status) => set({ saveStatus: status }),
  setLastSavedAt: (time) => set({ lastSavedAt: time }),
  setIsDirty: (dirty) => set({ isDirty: dirty }),
  setWordCount: (count) => set({ wordCount: count }),
  setAiStreamStatus: (status) => set({ aiStreamStatus: status }),
  setAiPreviewContent: (content) => set({ aiPreviewContent: content }),
  appendAiPreviewContent: (delta) =>
    set((s) => ({ aiPreviewContent: s.aiPreviewContent + delta })),
  setAiRequestId: (id) => set({ aiRequestId: id }),
  setAiTaskType: (taskType) => set({ aiTaskType: taskType }),
  resetAiPreview: () =>
    set({
      aiStreamStatus: "idle",
      aiPreviewContent: "",
      aiRequestId: null,
      aiTaskType: ""
    }),
  reset: () => set(initialState)
}));
