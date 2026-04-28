import { create } from "zustand";
import type { SkillManifest } from "../api/skillsApi.js";

export interface SkillStoreState {
  skills: SkillManifest[];
  selectedId: string | null;
  editingContent: string | null;
  saving: boolean;
  loading: boolean;
  error: string | null;
  setSkills: (skills: SkillManifest[]) => void;
  setSelectedId: (id: string | null) => void;
  setEditingContent: (content: string | null) => void;
  setSaving: (v: boolean) => void;
  setLoading: (v: boolean) => void;
  setError: (msg: string | null) => void;
}

export const useSkillStore = create<SkillStoreState>((set) => ({
  skills: [],
  selectedId: null,
  editingContent: null,
  saving: false,
  loading: false,
  error: null,
  setSkills: (skills) => set({ skills }),
  setSelectedId: (id) => set({ selectedId: id, editingContent: null, error: null }),
  setEditingContent: (content) => set({ editingContent: content }),
  setSaving: (v) => set({ saving: v }),
  setLoading: (v) => set({ loading: v }),
  setError: (msg) => set({ error: msg }),
}));
