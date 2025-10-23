-- This file should undo anything in `up.sql`
-- Drop active column from users table
ALTER TABLE users DROP COLUMN active;
