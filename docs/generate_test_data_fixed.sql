-- Generate 1 million test records for performance testing (Fixed version)
-- This script respects the phone varchar(11) constraint

-- First, let's clear any existing test data (optional - uncomment if needed)
-- DELETE FROM people WHERE id > 1;

-- Generate 1 million records efficiently using a single INSERT with generate_series
WITH generated_data AS (
  SELECT 
    i,
    (ARRAY['John', 'Jane', 'Michael', 'Sarah', 'David', 'Emma', 'Chris', 'Lisa', 'James', 'Maria', 
           'Robert', 'Jennifer', 'William', 'Linda', 'Richard', 'Elizabeth', 'Charles', 'Barbara', 
           'Joseph', 'Susan', 'Thomas', 'Jessica', 'Daniel', 'Karen', 'Matthew', 'Nancy', 'Anthony', 
           'Betty', 'Mark', 'Helen', 'Paul', 'Sandra', 'Steven', 'Donna', 'Andrew', 'Carol', 
           'Joshua', 'Ruth', 'Kenneth', 'Sharon', 'Kevin', 'Michelle', 'Brian', 'Laura', 'George', 
           'Edward', 'Ronald', 'Timothy', 'Jason', 'Jeffrey', 'Ryan', 'Jacob', 'Gary', 'Nicholas', 
           'Eric', 'Jonathan', 'Stephen', 'Larry', 'Justin', 'Scott', 'Brandon', 'Benjamin', 'Samuel', 
           'Gregory', 'Frank', 'Raymond', 'Alexander', 'Patrick', 'Jack', 'Dennis', 'Jerry'])[1 + (i % 70)] 
    || ' ' || 
    (ARRAY['Smith', 'Johnson', 'Williams', 'Brown', 'Jones', 'Garcia', 'Miller', 'Davis', 'Rodriguez', 
           'Martinez', 'Hernandez', 'Lopez', 'Gonzalez', 'Wilson', 'Anderson', 'Thomas', 'Taylor', 
           'Moore', 'Jackson', 'Martin', 'Lee', 'Perez', 'Thompson', 'White', 'Harris', 'Sanchez', 
           'Clark', 'Ramirez', 'Lewis', 'Robinson', 'Walker', 'Young', 'Allen', 'King', 'Wright', 
           'Scott', 'Torres', 'Nguyen', 'Hill', 'Flores', 'Green', 'Adams', 'Nelson', 'Baker', 
           'Hall', 'Rivera', 'Campbell', 'Mitchell', 'Carter', 'Roberts'])[1 + (i % 50)] as name,
    
    (i % 2 = 0) as gender,  -- Alternating gender
    
    (18 + (i % 62)) as age,  -- Age between 18 and 79
    
    (i % 999 + 1)::text || ' ' ||
    (ARRAY['Main St', 'Oak Ave', 'Pine Rd', 'Elm Dr', 'Maple Ln', 'Cedar Blvd', 'Park Ave', 
           'First St', 'Second St', 'Third Ave'])[1 + (i % 10)] || ', ' ||
    (ARRAY['New York', 'Los Angeles', 'Chicago', 'Houston', 'Phoenix', 'Philadelphia', 
           'San Antonio', 'San Diego', 'Dallas', 'San Jose'])[1 + (i % 10)] || ', ' ||
    (ARRAY['NY', 'CA', 'IL', 'TX', 'AZ', 'PA', 'FL', 'OH', 'NC', 'WA'])[1 + (i % 10)] || ' ' ||
    lpad((10000 + (i % 89999))::text, 5, '0') as address,
    
    -- Phone number exactly 11 chars: format like 12345678901
    lpad((2000000000 + (i % 7999999999))::text, 11, '0') as phone,
    
    lower(
      (ARRAY['john', 'jane', 'mike', 'sara', 'dave', 'emma', 'chris', 'lisa', 'jim', 'mary'])[1 + (i % 10)] ||
      (i % 1000)::text || '@' ||
      (ARRAY['gmail.com', 'yahoo.com', 'hotmail.com', 'test.com', 'example.org'])[1 + (i % 5)]
    ) as email
  FROM generate_series(1, 1000000) as s(i)
)
INSERT INTO people (name, gender, age, address, phone, email)
SELECT name, gender, age, address, phone, email FROM generated_data;

-- Display statistics
SELECT 
    'Data Generation Complete!' as status,
    COUNT(*) as total_records,
    MIN(age) as min_age,
    MAX(age) as max_age,
    ROUND(AVG(age), 2) as avg_age,
    COUNT(CASE WHEN gender = true THEN 1 END) as male_count,
    COUNT(CASE WHEN gender = false THEN 1 END) as female_count
FROM people;

-- Show some sample records
SELECT 'Sample Records:' as info;
SELECT id, name, gender, age, LEFT(address, 30) as address_preview, phone, LEFT(email, 25) as email_preview 
FROM people 
ORDER BY id DESC 
LIMIT 10;