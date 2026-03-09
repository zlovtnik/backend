DO $$
DECLARE
    invalid_values TEXT;
BEGIN
    SELECT string_agg(DISTINCT gender::text, ', ')
    INTO invalid_values
    FROM people
    WHERE gender::text NOT IN ('male', 'female');

    IF invalid_values IS NOT NULL THEN
        RAISE EXCEPTION 'Unexpected gender enum values found in people.gender: %', invalid_values;
    END IF;
END
$$;

ALTER TABLE people
ALTER COLUMN gender TYPE BOOLEAN
USING CASE
    WHEN gender = 'male'::gender THEN TRUE
    WHEN gender = 'female'::gender THEN FALSE
    ELSE NULL
END;

DROP TYPE IF EXISTS gender;
