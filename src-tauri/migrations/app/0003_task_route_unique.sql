-- Canonicalize app-level llm_task_routes.task_type values, deduplicate legacy aliases,
-- then enforce one route per canonical task_type.

WITH ranked AS (
  SELECT
    id,
    CASE task_type
      WHEN 'chapter_draft' THEN 'chapter.draft'
      WHEN 'generate_chapter_draft' THEN 'chapter.draft'
      WHEN 'draft' THEN 'chapter.draft'
      WHEN 'chapter_continue' THEN 'chapter.continue'
      WHEN 'continue_chapter' THEN 'chapter.continue'
      WHEN 'continue_draft' THEN 'chapter.continue'
      WHEN 'chapter_rewrite' THEN 'chapter.rewrite'
      WHEN 'rewrite_selection' THEN 'chapter.rewrite'
      WHEN 'chapter_plan' THEN 'chapter.plan'
      WHEN 'plan_chapter' THEN 'chapter.plan'
      WHEN 'prose_naturalize' THEN 'prose.naturalize'
      WHEN 'deai_text' THEN 'prose.naturalize'
      WHEN 'character_create' THEN 'character.create'
      WHEN 'world.generate' THEN 'world.create_rule'
      WHEN 'world_create_rule' THEN 'world.create_rule'
      WHEN 'plot.generate' THEN 'plot.create_node'
      WHEN 'plot_create_node' THEN 'plot.create_node'
      WHEN 'scan_consistency' THEN 'consistency.scan'
      WHEN 'consistency_scan' THEN 'consistency.scan'
      WHEN 'generate_blueprint_step' THEN 'blueprint.generate_step'
      WHEN 'blueprint_generate' THEN 'blueprint.generate_step'
      ELSE task_type
    END AS canonical_task_type,
    updated_at,
    created_at,
    rowid,
    ROW_NUMBER() OVER (
      PARTITION BY
        CASE task_type
          WHEN 'chapter_draft' THEN 'chapter.draft'
          WHEN 'generate_chapter_draft' THEN 'chapter.draft'
          WHEN 'draft' THEN 'chapter.draft'
          WHEN 'chapter_continue' THEN 'chapter.continue'
          WHEN 'continue_chapter' THEN 'chapter.continue'
          WHEN 'continue_draft' THEN 'chapter.continue'
          WHEN 'chapter_rewrite' THEN 'chapter.rewrite'
          WHEN 'rewrite_selection' THEN 'chapter.rewrite'
          WHEN 'chapter_plan' THEN 'chapter.plan'
          WHEN 'plan_chapter' THEN 'chapter.plan'
          WHEN 'prose_naturalize' THEN 'prose.naturalize'
          WHEN 'deai_text' THEN 'prose.naturalize'
          WHEN 'character_create' THEN 'character.create'
          WHEN 'world.generate' THEN 'world.create_rule'
          WHEN 'world_create_rule' THEN 'world.create_rule'
          WHEN 'plot.generate' THEN 'plot.create_node'
          WHEN 'plot_create_node' THEN 'plot.create_node'
          WHEN 'scan_consistency' THEN 'consistency.scan'
          WHEN 'consistency_scan' THEN 'consistency.scan'
          WHEN 'generate_blueprint_step' THEN 'blueprint.generate_step'
          WHEN 'blueprint_generate' THEN 'blueprint.generate_step'
          ELSE task_type
        END
      ORDER BY updated_at DESC, created_at DESC, rowid DESC
    ) AS rn
  FROM llm_task_routes
)
UPDATE llm_task_routes
SET task_type = (
  SELECT canonical_task_type
  FROM ranked
  WHERE ranked.id = llm_task_routes.id
)
WHERE id IN (SELECT id FROM ranked);

WITH ranked AS (
  SELECT
    id,
    ROW_NUMBER() OVER (
      PARTITION BY task_type
      ORDER BY updated_at DESC, created_at DESC, rowid DESC
    ) AS rn
  FROM llm_task_routes
)
DELETE FROM llm_task_routes
WHERE id IN (SELECT id FROM ranked WHERE rn > 1);

CREATE UNIQUE INDEX IF NOT EXISTS ux_llm_task_routes_task_type
ON llm_task_routes(task_type);
