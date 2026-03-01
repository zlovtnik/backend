ALTER TABLE people
ALTER COLUMN gender TYPE BOOLEAN
USING CASE
    WHEN gender = 'male'::gender THEN TRUE
    WHEN gender = 'female'::gender THEN FALSE
    ELSE NULL
END;

DROP TYPE IF EXISTS gender;
