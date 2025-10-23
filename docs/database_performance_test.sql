-- Direct database performance test queries for 1M records

-- First, let's see the actual record count
SELECT 'Current record count:' as info, COUNT(*) as total_records FROM people;

-- Test 1: First page performance (should be very fast)
SELECT 'Test 1: First page (LIMIT 50, OFFSET 0)' as test_name;
EXPLAIN ANALYZE
SELECT id, name, gender, age, LEFT(address, 20) as address_short, phone, LEFT(email, 25) as email_short
FROM people 
ORDER BY id 
LIMIT 50 OFFSET 0;

-- Test 2: Page 100 performance 
SELECT 'Test 2: Page 100 (LIMIT 50, OFFSET 5000)' as test_name;
EXPLAIN ANALYZE
SELECT id, name, gender, age, LEFT(address, 20) as address_short, phone, LEFT(email, 25) as email_short
FROM people 
ORDER BY id 
LIMIT 50 OFFSET 5000;

-- Test 3: Page 1000 performance (getting slower)
SELECT 'Test 3: Page 1000 (LIMIT 50, OFFSET 50000)' as test_name;
EXPLAIN ANALYZE
SELECT id, name, gender, age, LEFT(address, 20) as address_short, phone, LEFT(email, 25) as email_short
FROM people 
ORDER BY id 
LIMIT 50 OFFSET 50000;

-- Test 4: Page 10000 performance (quite slow)
SELECT 'Test 4: Page 10000 (LIMIT 50, OFFSET 500000)' as test_name;
EXPLAIN ANALYZE
SELECT id, name, gender, age, LEFT(address, 20) as address_short, phone, LEFT(email, 25) as email_short
FROM people 
ORDER BY id 
LIMIT 50 OFFSET 500000;

-- Test 5: Cursor-based pagination (much better for large offsets)
SELECT 'Test 5: Cursor-based pagination (WHERE id > 500000)' as test_name;
EXPLAIN ANALYZE
SELECT id, name, gender, age, LEFT(address, 20) as address_short, phone, LEFT(email, 25) as email_short
FROM people 
WHERE id > 500000
ORDER BY id 
LIMIT 50;

-- Test 6: Count query performance
SELECT 'Test 6: Count query performance' as test_name;
EXPLAIN ANALYZE SELECT COUNT(*) FROM people;

-- Test 7: Filtered query performance (age range)
SELECT 'Test 7: Age filter performance' as test_name;
EXPLAIN ANALYZE
SELECT id, name, gender, age, LEFT(address, 20) as address_short, phone, LEFT(email, 25) as email_short
FROM people 
WHERE age BETWEEN 25 AND 35
ORDER BY id 
LIMIT 50;

-- Test 8: Name search performance
SELECT 'Test 8: Name search performance' as test_name;
EXPLAIN ANALYZE
SELECT id, name, gender, age, LEFT(address, 20) as address_short, phone, LEFT(email, 25) as email_short
FROM people 
WHERE name ILIKE '%John%'
ORDER BY id 
LIMIT 50;

-- Test 9: Complex query with multiple filters
SELECT 'Test 9: Complex multi-filter query' as test_name;
EXPLAIN ANALYZE
SELECT id, name, gender, age, LEFT(address, 20) as address_short, phone, LEFT(email, 25) as email_short
FROM people 
WHERE age BETWEEN 25 AND 35 
  AND gender = true 
  AND name ILIKE '%a%'
ORDER BY id 
LIMIT 50;

-- Performance recommendations based on results
SELECT '=== PERFORMANCE ANALYSIS ===' as info;
SELECT 'Index status:' as info;
SELECT 
    schemaname, 
    tablename, 
    indexname, 
    indexdef 
FROM pg_indexes 
WHERE tablename = 'people';

SELECT 'Table statistics:' as info;
SELECT 
    schemaname,
    tablename,
    n_tup_ins as rows_inserted,
    n_tup_upd as rows_updated,
    n_tup_del as rows_deleted,
    n_live_tup as live_rows,
    n_dead_tup as dead_rows,
    last_vacuum,
    last_autovacuum,
    last_analyze,
    last_autoanalyze
FROM pg_stat_user_tables 
WHERE relname = 'people';