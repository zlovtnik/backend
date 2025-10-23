-- Change login_history.user_id from BIGINT to INTEGER to match user.id type
ALTER TABLE login_history ALTER COLUMN user_id TYPE INTEGER;
