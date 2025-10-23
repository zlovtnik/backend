-- Performance testing queries for pagination with 1M records

-- Test 1: Basic pagination performance (first page)
EXPLAIN ANALYZE
SELECT id, name, gender, age, address, phone, email 
FROM people 
ORDER BY id 
LIMIT 50 OFFSET 0;

-- Test 2: Middle pagination performance (page 10,000)
EXPLAIN ANALYZE
SELECT id, name, gender, age, address, phone, email 
FROM people 
ORDER BY id 
LIMIT 50 OFFSET 500000;

-- Test 3: Last page performance (worst case)
EXPLAIN ANALYZE
SELECT id, name, gender, age, address, phone, email 
FROM people 
ORDER BY id 
LIMIT 50 OFFSET 999950;

-- Test 4: Count query performance (total records)
EXPLAIN ANALYZE
SELECT COUNT(*) FROM people;

-- Test 5: Filtered pagination (with WHERE clause)
EXPLAIN ANALYZE
SELECT id, name, gender, age, address, phone, email 
FROM people 
WHERE age BETWEEN 25 AND 35
ORDER BY id 
LIMIT 50 OFFSET 1000;

-- Test 6: Search by name (pattern matching)
EXPLAIN ANALYZE
SELECT id, name, gender, age, address, phone, email 
FROM people 
WHERE name ILIKE '%John%'
ORDER BY id 
LIMIT 50;

-- Test 7: Multi-column sorting
EXPLAIN ANALYZE
SELECT id, name, gender, age, address, phone, email 
FROM people 
ORDER BY age DESC, name ASC 
LIMIT 50 OFFSET 1000;

-- Test 8: Cursor-based pagination (more efficient for large offsets)
-- Get first batch
SELECT id, name, gender, age, address, phone, email 
FROM people 
WHERE id > 0
ORDER BY id 
LIMIT 50;

-- Get next batch using cursor (replace 50 with last id from previous query)
EXPLAIN ANALYZE
SELECT id, name, gender, age, address, phone, email 
FROM people 
WHERE id > 50
ORDER BY id 
LIMIT 50;

-- Performance comparison: Offset vs Cursor at position 500,000
-- Offset method (slow)
EXPLAIN ANALYZE
SELECT id, name, gender, age, address, phone, email 
FROM people 
ORDER BY id 
LIMIT 50 OFFSET 500000;

-- Cursor method (fast)
EXPLAIN ANALYZE
SELECT id, name, gender, age, address, phone, email 
FROM people 
WHERE id > 500050
ORDER BY id 
LIMIT 50;

-- Additional indexes for better performance
CREATE INDEX IF NOT EXISTS idx_people_age_id ON people(age, id);
CREATE INDEX IF NOT EXISTS idx_people_name_gin ON people USING gin(to_tsvector('english', name));

-- Analyze table statistics for query planner
ANALYZE people;