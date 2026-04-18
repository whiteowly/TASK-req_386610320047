-- Revert: remove tag-aware search vector trigger and function
DROP TRIGGER IF EXISTS trig_item_version_tags_search_vector ON item_version_tags;
DROP FUNCTION IF EXISTS trg_update_search_vector_on_tag_change();
DROP FUNCTION IF EXISTS compute_search_vector(UUID);

-- Rebuild search vectors without tags (original title+body only)
UPDATE item_versions SET search_vector =
    setweight(to_tsvector('english', COALESCE(title, '')), 'A')
    || setweight(to_tsvector('english', COALESCE(body, '')), 'B');
