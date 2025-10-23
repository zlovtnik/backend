-- Add unique constraint on email column
-- First, check for existing duplicate emails
DO $$
DECLARE
    duplicate_count INTEGER;
BEGIN
    -- Count duplicate emails
    SELECT COUNT(*) INTO duplicate_count
    FROM (
        SELECT email
        FROM users 
        WHERE email IS NOT NULL AND email != ''
        GROUP BY email 
        HAVING COUNT(*) > 1
    ) AS duplicates;
    
    -- If duplicates exist, abort the migration with a clear error message
    IF duplicate_count > 0 THEN
        RAISE EXCEPTION 'Migration aborted: Found % duplicate email(s) in users table. Please resolve duplicate emails before applying this constraint. Run this query to find duplicates: SELECT email, COUNT(*) FROM users WHERE email IS NOT NULL AND email != '''' GROUP BY email HAVING COUNT(*) > 1;', duplicate_count;
    END IF;
    
    -- Log success message
    RAISE NOTICE 'No duplicate emails found. Proceeding with UNIQUE constraint.';
END $$;

-- Add the unique constraint (only reached if no duplicates exist)
ALTER TABLE users ADD CONSTRAINT users_email_unique UNIQUE (email);
