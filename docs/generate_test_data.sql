-- Generate 1 million test records for performance testing
-- This script uses PostgreSQL's generate_series() and random functions for efficiency

-- Create a function to generate random names
CREATE OR REPLACE FUNCTION random_name() RETURNS TEXT AS $$
DECLARE
    first_names TEXT[] := ARRAY['John', 'Jane', 'Michael', 'Sarah', 'David', 'Emma', 'Chris', 'Lisa', 'James', 'Maria', 'Robert', 'Jennifer', 'William', 'Linda', 'Richard', 'Elizabeth', 'Charles', 'Barbara', 'Joseph', 'Susan', 'Thomas', 'Jessica', 'Daniel', 'Karen', 'Matthew', 'Nancy', 'Anthony', 'Betty', 'Mark', 'Helen', 'Paul', 'Sandra', 'Steven', 'Donna', 'Andrew', 'Carol', 'Joshua', 'Ruth', 'Kenneth', 'Sharon', 'Kevin', 'Michelle', 'Brian', 'Laura', 'George', 'Sarah', 'Edward', 'Kimberly', 'Ronald', 'Deborah', 'Timothy', 'Dorothy', 'Jason', 'Lisa', 'Jeffrey', 'Nancy', 'Ryan', 'Karen', 'Jacob', 'Betty', 'Gary', 'Helen', 'Nicholas', 'Sandra', 'Eric', 'Donna', 'Jonathan', 'Carol', 'Stephen', 'Ruth', 'Larry', 'Sharon', 'Justin', 'Michelle', 'Scott', 'Laura', 'Brandon', 'Sarah', 'Benjamin', 'Kimberly', 'Samuel', 'Deborah', 'Gregory', 'Dorothy', 'Frank', 'Lisa', 'Raymond', 'Nancy', 'Alexander', 'Karen', 'Patrick', 'Betty', 'Jack', 'Helen', 'Dennis', 'Sandra', 'Jerry', 'Donna'];
    last_names TEXT[] := ARRAY['Smith', 'Johnson', 'Williams', 'Brown', 'Jones', 'Garcia', 'Miller', 'Davis', 'Rodriguez', 'Martinez', 'Hernandez', 'Lopez', 'Gonzalez', 'Wilson', 'Anderson', 'Thomas', 'Taylor', 'Moore', 'Jackson', 'Martin', 'Lee', 'Perez', 'Thompson', 'White', 'Harris', 'Sanchez', 'Clark', 'Ramirez', 'Lewis', 'Robinson', 'Walker', 'Young', 'Allen', 'King', 'Wright', 'Scott', 'Torres', 'Nguyen', 'Hill', 'Flores', 'Green', 'Adams', 'Nelson', 'Baker', 'Hall', 'Rivera', 'Campbell', 'Mitchell', 'Carter', 'Roberts'];
BEGIN
    RETURN first_names[1 + floor(random() * array_length(first_names, 1))] || ' ' || 
           last_names[1 + floor(random() * array_length(last_names, 1))];
END;
$$ LANGUAGE plpgsql;

-- Create a function to generate random addresses
CREATE OR REPLACE FUNCTION random_address() RETURNS TEXT AS $$
DECLARE
    street_numbers TEXT[] := ARRAY['123', '456', '789', '321', '654', '987', '147', '258', '369', '741'];
    street_names TEXT[] := ARRAY['Main St', 'Oak Ave', 'Pine Rd', 'Elm Dr', 'Maple Ln', 'Cedar Blvd', 'Park Ave', 'First St', 'Second St', 'Third Ave', 'Fourth St', 'Fifth Ave', 'Sixth St', 'Seventh Ave', 'Eighth St', 'Ninth Ave', 'Tenth St'];
    cities TEXT[] := ARRAY['New York', 'Los Angeles', 'Chicago', 'Houston', 'Phoenix', 'Philadelphia', 'San Antonio', 'San Diego', 'Dallas', 'San Jose', 'Austin', 'Jacksonville', 'Fort Worth', 'Columbus', 'Charlotte', 'San Francisco', 'Indianapolis', 'Seattle', 'Denver', 'Washington', 'Boston', 'El Paso', 'Nashville', 'Detroit', 'Oklahoma City'];
    states TEXT[] := ARRAY['NY', 'CA', 'IL', 'TX', 'AZ', 'PA', 'FL', 'OH', 'NC', 'WA', 'CO', 'DC', 'MA', 'TN', 'MI', 'OK'];
BEGIN
    RETURN street_numbers[1 + floor(random() * array_length(street_numbers, 1))] || ' ' ||
           street_names[1 + floor(random() * array_length(street_names, 1))] || ', ' ||
           cities[1 + floor(random() * array_length(cities, 1))] || ', ' ||
           states[1 + floor(random() * array_length(states, 1))] || ' ' ||
           lpad(floor(random() * 99999)::text, 5, '0');
END;
$$ LANGUAGE plpgsql;

-- Create a function to generate random phone numbers
CREATE OR REPLACE FUNCTION random_phone() RETURNS TEXT AS $$
BEGIN
    RETURN '(' || lpad(floor(random() * 900 + 100)::text, 3, '0') || ') ' ||
           lpad(floor(random() * 900 + 100)::text, 3, '0') || '-' ||
           lpad(floor(random() * 9000 + 1000)::text, 4, '0');
END;
$$ LANGUAGE plpgsql;

-- Create a function to generate random email addresses
CREATE OR REPLACE FUNCTION random_email(name_input TEXT) RETURNS TEXT AS $$
DECLARE
    domains TEXT[] := ARRAY['gmail.com', 'yahoo.com', 'hotmail.com', 'outlook.com', 'aol.com', 'icloud.com', 'protonmail.com', 'company.com', 'test.org', 'example.net'];
    clean_name TEXT;
BEGIN
    -- Clean the name: lowercase, replace spaces with dots, remove special characters
    clean_name := lower(regexp_replace(name_input, '[^a-zA-Z0-9\s]', '', 'g'));
    clean_name := replace(clean_name, ' ', '.');
    
    RETURN clean_name || floor(random() * 1000)::text || '@' || 
           domains[1 + floor(random() * array_length(domains, 1))];
END;
$$ LANGUAGE plpgsql;

-- Check current record count
SELECT COUNT(*) as current_count FROM people;

-- Generate 1 million records in batches for better performance
-- Using INSERT with generate_series for maximum efficiency
INSERT INTO people (name, gender, age, address, phone, email)
SELECT 
    random_name() as name,
    (random() > 0.5) as gender,
    (18 + floor(random() * 62))::integer as age,  -- Age between 18 and 79
    random_address() as address,
    random_phone() as phone,
    random_email(random_name()) as email
FROM generate_series(1, 1000000) as s(i);

-- Check final record count
SELECT COUNT(*) as final_count FROM people;

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_people_name ON people(name);
CREATE INDEX IF NOT EXISTS idx_people_age ON people(age);
CREATE INDEX IF NOT EXISTS idx_people_email ON people(email);

-- Clean up the temporary functions
DROP FUNCTION IF EXISTS random_name();
DROP FUNCTION IF EXISTS random_address();
DROP FUNCTION IF EXISTS random_phone();
DROP FUNCTION IF EXISTS random_email(TEXT);

-- Display some sample data
SELECT 'Sample of generated data:' as info;
SELECT id, name, gender, age, address, phone, email 
FROM people 
ORDER BY id DESC 
LIMIT 10;

-- Performance statistics
SELECT 
    'Performance Statistics:' as info,
    COUNT(*) as total_records,
    MIN(age) as min_age,
    MAX(age) as max_age,
    AVG(age) as avg_age,
    COUNT(CASE WHEN gender = true THEN 1 END) as male_count,
    COUNT(CASE WHEN gender = false THEN 1 END) as female_count
FROM people;