-- This file should undo anything in `up.sql`
-- Revert login_history.user_id back to BIGINT
ALTER TABLE login_history ALTER COLUMN user_id TYPE BIGINT;
