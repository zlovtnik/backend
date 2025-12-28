-- Revert nfag_dest timestamp constraints and defaults
ALTER TABLE nfag_dest
    ALTER COLUMN created_at DROP NOT NULL,
    ALTER COLUMN updated_at DROP NOT NULL,
    ALTER COLUMN created_at DROP DEFAULT,
    ALTER COLUMN updated_at DROP DEFAULT;
