import { useQuery } from "@tanstack/react-query";
import { getDashboardStats } from "../api/statsApi.js";

export function useDashboardStats(projectRoot: string | null) {
  return useQuery({
    queryKey: ["project", "stats", projectRoot ?? ""],
    queryFn: () => getDashboardStats(projectRoot!),
    enabled: !!projectRoot,
  });
}
