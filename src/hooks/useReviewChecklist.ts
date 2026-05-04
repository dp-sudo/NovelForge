import { useState, useCallback } from "react";
import type { ModuleAiTaskResult } from "../api/moduleAiApi.js";

export function useReviewChecklist() {
  const [reviewChecklistHints, setReviewChecklistHints] = useState<string[]>([]);
  const [reviewWorkItemCount, setReviewWorkItemCount] = useState(0);
  const [taskContractHint, setTaskContractHint] = useState<string | null>(null);

  const processReviewResult = useCallback((result: ModuleAiTaskResult) => {
    const hints = result.reviewChecklist
      .filter((item) => item.status === "attention")
      .map((item) => `${item.title}: ${item.message}`);
    setReviewChecklistHints(hints);
    setReviewWorkItemCount(result.reviewWorkItems.length);
    
    const contract = result.taskContract;
    if (contract) {
      const authorityLayer = typeof contract.authorityLayer === "string" ? contract.authorityLayer : "n/a";
      const stateLayer = typeof contract.stateLayer === "string" ? contract.stateLayer : "n/a";
      const capabilityPack = typeof contract.capabilityPack === "string" ? contract.capabilityPack : "n/a";
      const reviewGate = typeof contract.reviewGate === "string" ? contract.reviewGate : "n/a";
      setTaskContractHint(`权威层: ${authorityLayer} | 状态层: ${stateLayer} | 能力包: ${capabilityPack} | 审查门: ${reviewGate}`);
    } else {
      setTaskContractHint(null);
    }
  }, []);

  const resetReview = useCallback(() => {
    setReviewChecklistHints([]);
    setReviewWorkItemCount(0);
    setTaskContractHint(null);
  }, []);

  return {
    reviewChecklistHints,
    reviewWorkItemCount,
    taskContractHint,
    processReviewResult,
    resetReview
  };
}
