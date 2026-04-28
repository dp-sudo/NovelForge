// Simple line-based diff view for comparing original and rewritten text.
// Uses a basic LCS (Longest Common Subsequence) algorithm on lines.

interface DiffLine {
  type: "add" | "del" | "same";
  text: string;
}

function computeDiff(original: string, revised: string): DiffLine[] {
  const origLines = original.split("\n");
  const revLines = revised.split("\n");

  // Simple LCS-based diff on lines
  const m = origLines.length;
  const n = revLines.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (origLines[i - 1] === revLines[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  // Backtrack to find the diff
  const result: DiffLine[] = [];
  let i = m, j = n;
  const temp: DiffLine[] = [];

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && origLines[i - 1] === revLines[j - 1]) {
      temp.push({ type: "same", text: origLines[i - 1] });
      i--; j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      temp.push({ type: "add", text: revLines[j - 1] });
      j--;
    } else {
      temp.push({ type: "del", text: origLines[i - 1] });
      i--;
    }
  }

  // Reverse to correct order
  for (let k = temp.length - 1; k >= 0; k--) {
    result.push(temp[k]);
  }

  return result;
}

interface DiffViewProps {
  original: string;
  revised: string;
}

export function DiffView({ original, revised }: DiffViewProps) {
  const lines = computeDiff(original, revised);

  if (!original.trim()) {
    return <pre className="text-sm text-surface-200 whitespace-pre-wrap font-sans leading-relaxed">{revised}</pre>;
  }

  return (
    <div className="font-mono text-sm leading-relaxed">
      {lines.map((line, idx) => {
        const baseClass = "px-2 py-0.5 whitespace-pre-wrap";
        if (line.type === "add") {
          return (
            <div key={idx} className={`${baseClass} bg-success/10 text-success-300 border-l-2 border-success`}>
              <span className="select-none mr-2 opacity-60">+</span>
              {line.text || " "}
            </div>
          );
        }
        if (line.type === "del") {
          return (
            <div key={idx} className={`${baseClass} bg-error/10 text-error-300 border-l-2 border-error line-through`}>
              <span className="select-none mr-2 opacity-60">−</span>
              {line.text || " "}
            </div>
          );
        }
        return (
          <div key={idx} className={`${baseClass} text-surface-400 border-l-2 border-transparent`}>
            <span className="select-none mr-2 opacity-30" />
            {line.text}
          </div>
        );
      })}
    </div>
  );
}
