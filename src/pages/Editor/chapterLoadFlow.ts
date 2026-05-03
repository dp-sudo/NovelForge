export interface ChapterDraftRecoveryResult {
  hasNewerDraft: boolean;
  draftContent?: string;
}

export interface LoadEditorChapterInput {
  chapterId: string;
  projectRoot: string;
  readChapterContent: (chapterId: string, projectRoot: string) => Promise<string>;
  recoverDraft: (chapterId: string, projectRoot: string) => Promise<ChapterDraftRecoveryResult>;
}

export interface LoadEditorChapterOutput {
  persistedContent: string;
  recoveryContent: string | null;
}

export async function loadEditorChapterContentWithRecovery(
  input: LoadEditorChapterInput,
): Promise<LoadEditorChapterOutput> {
  const { chapterId, projectRoot, readChapterContent, recoverDraft } = input;
  let persistedContent = "";
  try {
    persistedContent = await readChapterContent(chapterId, projectRoot);
  } catch {
    persistedContent = "";
  }

  try {
    const recovery = await recoverDraft(chapterId, projectRoot);
    if (
      recovery.hasNewerDraft
      && recovery.draftContent
      && recovery.draftContent !== persistedContent
    ) {
      return { persistedContent, recoveryContent: recovery.draftContent };
    }
  } catch {
  }

  return { persistedContent, recoveryContent: null };
}
