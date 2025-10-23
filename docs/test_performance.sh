#!/bin/bash

# Performance testing script for pagination with 1M records
# Make sure your Actix Web server is running before executing this script

echo "🚀 Starting pagination performance tests with 1M records..."
echo "=================================================="

# Test database query performance directly
echo "📊 Database Performance Tests:"
echo "------------------------------"

echo "1. Testing first page (OFFSET 0)..."
time psql "$DATABASE_URL" -c "SELECT id, name, age FROM people ORDER BY id LIMIT 50 OFFSET 0;" > /dev/null

echo "2. Testing middle page (OFFSET 500,000) - This will be slow..."
time psql "$DATABASE_URL" -c "SELECT id, name, age FROM people ORDER BY id LIMIT 50 OFFSET 500000;" > /dev/null

echo "3. Testing cursor-based approach (much faster)..."
time psql "$DATABASE_URL" -c "SELECT id, name, age FROM people WHERE id > 500000 ORDER BY id LIMIT 50;" > /dev/null

echo "4. Testing count query..."
time psql "$DATABASE_URL" -c "SELECT COUNT(*) FROM people;" > /dev/null

echo "5. Testing age-based filtering..."
time psql "$DATABASE_URL" -c "SELECT id, name, age FROM people WHERE age BETWEEN 25 AND 35 ORDER BY id LIMIT 50;" > /dev/null

echo ""
echo "📡 API Endpoint Tests (requires running server):"
echo "------------------------------------------------"

# Check if server is running
if curl -s http://localhost:8000/api/ping > /dev/null; then
    echo "✅ Server is running, testing API endpoints..."
    
    # Get auth token (you might need to adjust credentials)
    TOKEN_RESPONSE=$(curl -s -X POST http://localhost:8000/api/auth/login \
        -H "Content-Type: application/json" \
        -d '{"username":"admin","password":"password"}')
    
    if echo "$TOKEN_RESPONSE" | grep -q "token"; then
        TOKEN=$(echo "$TOKEN_RESPONSE" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
        echo "✅ Authentication successful"
        
        echo "1. Testing first page API call..."
        time curl -s "http://localhost:8000/api/address-book?page=1&per_page=50" \
            -H "Authorization: Bearer $TOKEN" > /dev/null
        
        echo "2. Testing page 1000 API call..."
        time curl -s "http://localhost:8000/api/address-book?page=1000&per_page=50" \
            -H "Authorization: Bearer $TOKEN" > /dev/null
        
        echo "3. Testing search API call..."
        time curl -s "http://localhost:8000/api/address-book?page=1&per_page=50&name=John" \
            -H "Authorization: Bearer $TOKEN" > /dev/null
        
        echo "4. Testing age filter API call..."
        time curl -s "http://localhost:8000/api/address-book?page=1&per_page=50&min_age=25&max_age=35" \
            -H "Authorization: Bearer $TOKEN" > /dev/null
    else
        echo "❌ Authentication failed. Check your credentials."
    fi
else
    echo "❌ Server is not running. Start your Actix Web server first:"
    echo "   cargo run"
fi

echo ""
echo "💡 Performance Recommendations:"
echo "================================"
echo "• OFFSET-based pagination becomes very slow with large offsets"
echo "• Consider implementing cursor-based pagination using 'WHERE id > last_id'"
echo "• Add appropriate indexes for filtered queries"
echo "• Use EXPLAIN ANALYZE to monitor query performance"
echo "• Consider implementing caching for frequently accessed pages"
echo ""
echo "🔍 To analyze query plans:"
echo "psql \"\$DATABASE_URL\" -c \"EXPLAIN ANALYZE SELECT * FROM people ORDER BY id LIMIT 50 OFFSET 500000;\""