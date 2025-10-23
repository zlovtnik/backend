-- This file should undo anything in `up.sql`
-- Drop unique constraint on email column
ALTER TABLE users DROP CONSTRAINT users_email_unique;
