DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM people WHERE gender IS NULL LIMIT 1) THEN
        RAISE EXCEPTION 'Rollback blocked: people.gender contains NULL values and converting to NOT NULL would lose semantic information.';
    END IF;
END $$;

ALTER TABLE people
ALTER COLUMN gender SET NOT NULL;
