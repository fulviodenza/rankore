-- Add migration script here
ALTER TABLE users ADD COLUMN hasLeft BOOLEAN DEFAULT FALSE;