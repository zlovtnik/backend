-- Add active field to users table
ALTER TABLE users ADD COLUMN active BOOLEAN NOT NULL DEFAULT TRUE;
