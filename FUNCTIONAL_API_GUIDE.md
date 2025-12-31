# Functional Operations API - React Integration Guide

This document describes the functional operations endpoints designed for React frontends to visualize functional programming patterns with animations and flux-style data flow.

## Overview

The Functional Operations API exposes four core functional programming patterns through REST endpoints. Each endpoint returns step-by-step transformation data suitable for animating the flow of data through functional pipelines.

**Base URL**: `/api/functional`

## Response Format

All endpoints return a `PipelineVisualization` object containing:

```typescript
{
  pipeline_id: string;           // Unique ID for this execution
  name: string;                  // Name of the demonstration
  steps: TransformationStep[];   // Array of transformation steps
  initial_data: number[];        // Original input data
  final_result: number[];        // Final output after all steps
  total_duration_ms: number;     // Total execution time
  executed_at: string;           // ISO 8601 timestamp
}
```

Each `TransformationStep` contains:

```typescript
{
  operation: string;       // Operation type (filter, map, etc.)
  description: string;     // Human-readable description
  input?: number[];        // Input data for this step
  output?: number[];       // Output data after this step
  duration_ms: number;     // Processing time for this step
}
```

## Endpoints

### 1. Get Available Operations

**Endpoint**: `GET /api/functional/operations`

Returns a list of all available functional demonstrations and their parameters.

**Response Example**:
```json
{
  "operations": [
    {
      "name": "Filter",
      "endpoint": "POST /api/functional/demo/filter",
      "description": "Filter data based on a condition",
      "conditions": ["even", "odd", "greater_than_5", "less_than_10"]
    },
    {
      "name": "Map",
      "endpoint": "POST /api/functional/demo/map",
      "description": "Transform each element using a function",
      "transformations": ["double", "square", "increment", "decrement", "absolute"]
    },
    {
      "name": "Chain",
      "endpoint": "POST /api/functional/demo/chain",
      "description": "Apply a sequence of operations",
      "operations": ["filter", "map", "take", "skip"]
    },
    {
      "name": "State Transitions",
      "endpoint": "POST /api/functional/demo/state-transitions",
      "description": "Demonstrate immutable state transitions",
      "mutations": ["increment", "decrement", "multiply", "divide", "set"]
    }
  ]
}
```

### 2. Filter Operation

**Endpoint**: `POST /api/functional/demo/filter`

Demonstrates filtering data based on a predicate condition.

**Request**:
```json
{
  "data": [1, 2, 3, 4, 5, 6],
  "condition": "even"
}
```

**Conditions Available**:
- `even` - Keep even numbers
- `odd` - Keep odd numbers
- `greater_than_5` - Keep numbers > 5
- `less_than_10` - Keep numbers < 10

**Response Example**:
```json
{
  "pipeline_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Filter Demo (even)",
  "steps": [
    {
      "operation": "filter",
      "description": "Filter by condition: even",
      "input": [1, 2, 3, 4, 5, 6],
      "output": [2, 4, 6],
      "duration_ms": 0
    }
  ],
  "initial_data": [1, 2, 3, 4, 5, 6],
  "final_result": [2, 4, 6],
  "total_duration_ms": 1,
  "executed_at": "2025-12-31T12:00:00+00:00"
}
```

### 3. Map Transformation

**Endpoint**: `POST /api/functional/demo/map`

Demonstrates transforming each element using a mapping function.

**Request**:
```json
{
  "data": [1, 2, 3, 4, 5],
  "transformation": "double"
}
```

**Transformations Available**:
- `double` - Multiply by 2
- `square` - Multiply by itself
- `increment` - Add 1
- `decrement` - Subtract 1
- `absolute` - Absolute value

**Response Example**:
```json
{
  "pipeline_id": "550e8400-e29b-41d4-a716-446655440001",
  "name": "Map Demo (double)",
  "steps": [
    {
      "operation": "map",
      "description": "Map transformation: double",
      "input": [1, 2, 3, 4, 5],
      "output": [2, 4, 6, 8, 10],
      "duration_ms": 0
    }
  ],
  "initial_data": [1, 2, 3, 4, 5],
  "final_result": [2, 4, 6, 8, 10],
  "total_duration_ms": 1,
  "executed_at": "2025-12-31T12:00:01+00:00"
}
```

### 4. Complex Chain

**Endpoint**: `POST /api/functional/demo/chain`

Demonstrates composing multiple operations in sequence, showing each step separately.

**Request**:
```json
{
  "data": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
  "operations": [
    {"op_type": "filter", "param": "even"},
    {"op_type": "map", "param": "double"},
    {"op_type": "take", "param": "2"}
  ]
}
```

**Operations Available**:
- `filter` (param: `even`, `odd`)
- `map` (param: `double`, `square`, `increment`)
- `take` (param: number of elements to take)
- `skip` (param: number of elements to skip)

**Response Example**:
```json
{
  "pipeline_id": "550e8400-e29b-41d4-a716-446655440002",
  "name": "Complex Chain Demonstration",
  "steps": [
    {
      "operation": "filter",
      "description": "Filter by: even",
      "input": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
      "output": [2, 4, 6, 8, 10],
      "duration_ms": 0
    },
    {
      "operation": "map",
      "description": "Map: double",
      "input": [2, 4, 6, 8, 10],
      "output": [4, 8, 12, 16, 20],
      "duration_ms": 0
    },
    {
      "operation": "take",
      "description": "Take first 2 elements",
      "input": [4, 8, 12, 16, 20],
      "output": [4, 8],
      "duration_ms": 0
    }
  ],
  "initial_data": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
  "final_result": [4, 8],
  "total_duration_ms": 1,
  "executed_at": "2025-12-31T12:00:02+00:00"
}
```

### 5. State Transitions

**Endpoint**: `POST /api/functional/demo/state-transitions`

Demonstrates immutable state transitions, showing how state evolves through mutations.

**Request**:
```json
{
  "initial_value": 10,
  "transitions": [
    {"mutation_type": "increment", "value": 5},
    {"mutation_type": "multiply", "value": 2},
    {"mutation_type": "decrement", "value": 3}
  ]
}
```

**Mutations Available**:
- `increment` - Add value
- `decrement` - Subtract value
- `multiply` - Multiply by value
- `divide` - Divide by value (skips if value is 0)
- `set` - Set to value

**Response Example**:
```json
{
  "pipeline_id": "550e8400-e29b-41d4-a716-446655440003",
  "name": "State Transition Demonstration",
  "steps": [
    {
      "operation": "increment",
      "description": "increment by 5 (10 → 15)",
      "input": [10],
      "output": [15],
      "duration_ms": 0
    },
    {
      "operation": "multiply",
      "description": "multiply by 2 (15 → 30)",
      "input": [15],
      "output": [30],
      "duration_ms": 0
    },
    {
      "operation": "decrement",
      "description": "decrement by 3 (30 → 27)",
      "input": [30],
      "output": [27],
      "duration_ms": 0
    }
  ],
  "initial_data": [10],
  "final_result": [27],
  "total_duration_ms": 1,
  "executed_at": "2025-12-31T12:00:03+00:00"
}
```

## React Integration Example

### Basic Hook for Functional Operations

```typescript
import { useState } from 'react';

interface TransformationStep {
  operation: string;
  description: string;
  input?: number[];
  output?: number[];
  duration_ms: number;
}

interface PipelineVisualization {
  pipeline_id: string;
  name: string;
  steps: TransformationStep[];
  initial_data: number[];
  final_result: number[];
  total_duration_ms: number;
  executed_at: string;
}

// Type for request payloads
interface DemoRequest {
  data?: number[];
  condition?: string;
  transformation?: string;
  operations?: { op_type: string; param: string }[];
  initial_value?: number;
  transitions?: { mutation_type: string; value: number }[];
}

const useFunctionalDemo = () => {
  const [visualization, setVisualization] = useState<PipelineVisualization | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Generic helper to reduce code duplication
  const runDemo = async (
    endpoint: string,
    payload: DemoRequest
  ): Promise<PipelineVisualization | null> => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });

      if (!response.ok) {
        const errorBody = await response.text();
        throw new Error(`HTTP ${response.status}: ${errorBody}`);
      }

      const result: PipelineVisualization = await response.json();
      setVisualization(result);
      return result;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown error';
      setError(message);
      return null;
    } finally {
      setLoading(false);
    }
  };

  // Individual operation runners (using generic helper)
  const runFilter = (data: number[], condition: string) =>
    runDemo('/api/functional/demo/filter', { data, condition });

  const runMap = (data: number[], transformation: string) =>
    runDemo('/api/functional/demo/map', { data, transformation });

  const runChain = (data: number[], operations: { op_type: string; param: string }[]) =>
    runDemo('/api/functional/demo/chain', { data, operations });

  const runStateTransitions = (initial_value: number, transitions: { mutation_type: string; value: number }[]) =>
    runDemo('/api/functional/demo/state-transitions', { initial_value, transitions });

  return {
    visualization,
    loading,
    error,
    runFilter,
    runMap,
    runChain,
    runStateTransitions,
  };
};

export default useFunctionalDemo;
```

**Key Improvements:**
1. **Generic `runDemo` helper**: All operations use a single fetch-with-error-handling pattern
2. **Explicit TypeScript types**: Interfaces for `PipelineVisualization`, `TransformationStep`, and `DemoRequest`
3. **Complete API coverage**: All 4 operations (filter, map, chain, state-transitions) exposed
4. **Better error handling**: HTTP error checking and clear error messages
5. **DRY principle**: No duplicate fetch logic between functions
6. **Type safety**: Return type `Promise<PipelineVisualization | null>` for all operations

### Animation Component Example

```typescript
import React, { useEffect, useState } from 'react';

interface TransformationStep {
  operation: string;
  description: string;
  input?: number[];
  output?: number[];
  duration_ms: number;
}

export const PipelineAnimator: React.FC<{ steps: TransformationStep[] }> = ({ steps }) => {
  const [currentStep, setCurrentStep] = useState(0);
  const [isAnimating, setIsAnimating] = useState(true);

  useEffect(() => {
    if (!isAnimating || currentStep >= steps.length) return;

    const timer = setTimeout(() => {
      setCurrentStep(prev => prev + 1);
    }, 1000); // 1 second per step

    return () => clearTimeout(timer);
  }, [currentStep, isAnimating, steps.length]);

  const step = steps[currentStep];

  return (
    <div className="pipeline-animator">
      <h3>{step?.operation}</h3>
      <p>{step?.description}</p>
      
      <div className="data-flow">
        {step?.input && (
          <div className="data-box">
            <label>Input:</label>
            <p>[{step.input.join(', ')}]</p>
          </div>
        )}
        <span className="arrow">→</span>
        {step?.output && (
          <div className="data-box">
            <label>Output:</label>
            <p>[{step.output.join(', ')}]</p>
          </div>
        )}
      </div>

      <div className="controls">
        <button onClick={() => setIsAnimating(!isAnimating)}>
          {isAnimating ? 'Pause' : 'Play'}
        </button>
        <button 
          onClick={() => setCurrentStep(Math.max(0, currentStep - 1))}
          disabled={currentStep === 0}
        >
          Previous
        </button>
        <button 
          onClick={() => setCurrentStep(Math.min(steps.length - 1, currentStep + 1))}
          disabled={currentStep === steps.length - 1}
        >
          Next
        </button>
        <span>{currentStep + 1} / {steps.length}</span>
      </div>
    </div>
  );
};
```

## Use Cases

### 1. Educational - Teach Functional Programming
Animate how pure functions transform data without side effects.

### 2. Data Flow Visualization
Show how data moves through filtering, mapping, and composition operations.

### 3. Performance Analysis
Display `duration_ms` per step to visualize which operations are expensive.

### 4. State Management Pattern Learning
Demonstrate immutable state transitions instead of in-place mutations.

## Error Handling

All endpoints return appropriate HTTP status codes:
- `200 OK` - Successful operation
- `400 Bad Request` - Invalid input (malformed request or unknown operation)
- `500 Internal Server Error` - Server-side error

Error responses include details in the response body.

## Testing

Run the included tests:

```bash
cargo test functional_operations_controller
```

## Future Enhancements

- [ ] Add custom function definitions
- [ ] Support async operations
- [ ] Add performance metrics and timing analysis
- [ ] Support lazy evaluation visualization
- [ ] Add error recovery visualization
- [ ] WebSocket real-time streaming for large datasets
