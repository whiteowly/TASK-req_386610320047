-- Migration: include tag text in item_versions.search_vector
-- Tags are stored in item_version_tags (linked after version INSERT),
-- so we need a function + trigger on item_version_tags to rebuild the vector.

-- Reusable function: computes search_vector from title (A), body (B), tags (C)
CREATE OR REPLACE FUNCTION compute_search_vector(p_version_id UUID)
RETURNS TSVECTOR
LANGUAGE plpgsql
AS $$
DECLARE
    v_title TEXT;
    v_body  TEXT;
    v_tags  TEXT;
BEGIN
    SELECT title, COALESCE(body, '')
      INTO v_title, v_body
      FROM item_versions
     WHERE id = p_version_id;

    SELECT COALESCE(string_agg(t.name, ' '), '')
      INTO v_tags
      FROM item_version_tags ivt
      JOIN tags t ON ivt.tag_id = t.id
     WHERE ivt.item_version_id = p_version_id;

    RETURN setweight(to_tsvector('english', COALESCE(v_title, '')), 'A')
        || setweight(to_tsvector('english', v_body), 'B')
        || setweight(to_tsvector('english', v_tags), 'C');
END;
$$;

-- Trigger: rebuild search_vector whenever tags are added/removed
CREATE OR REPLACE FUNCTION trg_update_search_vector_on_tag_change()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE item_versions
           SET search_vector = compute_search_vector(NEW.item_version_id)
         WHERE id = NEW.item_version_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE item_versions
           SET search_vector = compute_search_vector(OLD.item_version_id)
         WHERE id = OLD.item_version_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER trig_item_version_tags_search_vector
AFTER INSERT OR DELETE ON item_version_tags
FOR EACH ROW
EXECUTE FUNCTION trg_update_search_vector_on_tag_change();

-- Rebuild existing search vectors to include any already-linked tags
UPDATE item_versions SET search_vector = compute_search_vector(id);
