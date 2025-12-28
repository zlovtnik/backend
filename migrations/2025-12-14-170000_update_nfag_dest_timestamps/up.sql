-- Ensure nfag_dest timestamps have defaults, are backfilled, and remain non-null
ALTER TABLE nfag_dest
    ALTER COLUMN created_at SET DEFAULT NOW(),
    ALTER COLUMN updated_at SET DEFAULT NOW();

UPDATE nfag_dest
SET created_at = NOW()
WHERE created_at IS NULL;

UPDATE nfag_dest
SET updated_at = NOW()
WHERE updated_at IS NULL;

-- Safety check: fail the migration if any NULLs remain before tightening nullability
DO $$
DECLARE
    null_rows INT;
BEGIN
    SELECT COUNT(*) INTO null_rows FROM nfag_dest WHERE created_at IS NULL OR updated_at IS NULL;
    IF null_rows > 0 THEN
        RAISE EXCEPTION 'nfag_dest timestamps still contain % NULL rows; aborting NOT NULL alteration', null_rows;
    END IF;
END$$;

ALTER TABLE nfag_dest
    ALTER COLUMN created_at SET NOT NULL,
    ALTER COLUMN updated_at SET NOT NULL;
