DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'gender') THEN
        CREATE TYPE gender AS ENUM ('male', 'female', 'non_binary', 'prefer_not_to_say');
    END IF;
END $$;

ALTER TABLE people
ALTER COLUMN gender TYPE gender
USING CASE
    WHEN gender IS TRUE THEN 'male'::gender
    WHEN gender IS FALSE THEN 'female'::gender
    ELSE NULL
END;
