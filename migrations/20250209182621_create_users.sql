-- Create extension for UUID generation, if not already present.
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Drop the existing table if it exists.
DROP TABLE IF EXISTS users;

-- Create the new table with the correct columns.
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    hashed_password TEXT NOT NULL,
    paper_amount INTEGER NOT NULL DEFAULT 10000
);
