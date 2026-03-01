export type Direction = "asc" | "desc";

export type ActionType =
  | "filterEquals"
  | "filterGreaterThan"
  | "mapMultiply"
  | "sortBy"
  | "groupBy"
  | "take"
  | "skip"
  | "distinctBy";

export interface TransformationAction {
  id: string;
  type: ActionType;
  field: string;
  value: string;
  amount: number;
  direction: Direction;
}

export interface Snapshot {
  label: string;
  description: string;
  data: unknown;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function readPath(input: unknown, path: string): unknown {
  if (!path) {
    return input;
  }

  return path.split(".").reduce<unknown>((current, key) => {
    if (isRecord(current) || Array.isArray(current)) {
      return (current as Record<string, unknown>)[key];
    }
    return undefined;
  }, input);
}

function setPath(input: unknown, path: string, newValue: unknown): unknown {
  if (!path) {
    return newValue;
  }
  if (!isRecord(input)) {
    throw new TypeError("setPath expects a record object input.");
  }

  const keys = path.split(".");
  const root = { ...input };

  let cursor: Record<string, unknown> = root;
  for (let i = 0; i < keys.length - 1; i += 1) {
    const key = keys[i];
    const current = cursor[key];
    if (current === undefined) {
      cursor[key] = {};
    } else if (!isRecord(current)) {
      throw new TypeError(
        `setPath cannot traverse non-record segment '${key}' in path '${path}'.`
      );
    } else {
      cursor[key] = { ...current };
    }
    cursor = cursor[key] as Record<string, unknown>;
  }

  cursor[keys[keys.length - 1]] = newValue;
  return root;
}

function parseLooseLiteral(rawValue: string): unknown {
  const value = rawValue.trim();
  if (!value) {
    return "";
  }
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

function ensureArray(data: unknown, operation: ActionType): unknown[] {
  if (!Array.isArray(data)) {
    throw new Error(`${operation} expects array input but received non-array data.`);
  }
  return data;
}

function asNumber(value: unknown): number | null {
  const n = Number(value);
  return Number.isFinite(n) ? n : null;
}

function compare(left: unknown, right: unknown): boolean {
  if (left === right) {
    return true;
  }
  if (typeof left === "object" || typeof right === "object") {
    try {
      return JSON.stringify(left) === JSON.stringify(right);
    } catch {
      return false;
    }
  }
  return false;
}

function stableToken(value: unknown): string {
  if (value === null) {
    return "null:null";
  }
  if (value === undefined) {
    return "undefined:undefined";
  }

  const valueType = typeof value;
  if (valueType === "object") {
    try {
      if (Array.isArray(value)) {
        return `array:${JSON.stringify(value)}`;
      }

      const record = value as Record<string, unknown>;
      const sortedKeys = Object.keys(record).sort();
      const sortedRecord: Record<string, unknown> = {};
      for (const key of sortedKeys) {
        sortedRecord[key] = record[key];
      }
      return `object:${JSON.stringify(sortedRecord)}`;
    } catch {
      return "object:[unserializable]";
    }
  }
  return `${valueType}:${String(value)}`;
}

function clone<T>(value: T): T {
  if (typeof structuredClone === "function") {
    return structuredClone(value);
  }
  return JSON.parse(JSON.stringify(value)) as T;
}

export function describeData(data: unknown): string {
  if (Array.isArray(data)) {
    return `${data.length} item(s)`;
  }
  if (isRecord(data)) {
    return `${Object.keys(data).length} key(s)`;
  }
  return typeof data;
}

export function describeAction(action: TransformationAction): string {
  switch (action.type) {
    case "filterEquals":
      return `filter ${action.field || "(value)"} == ${action.value}`;
    case "filterGreaterThan":
      return `filter ${action.field || "(value)"} > ${action.amount}`;
    case "mapMultiply":
      return `map multiply ${action.field || "(value)"} by ${action.amount}`;
    case "sortBy":
      return `sort by ${action.field || "(value)"} ${action.direction}`;
    case "groupBy":
      return `group by ${action.field}`;
    case "take":
      return `take ${action.amount}`;
    case "skip":
      return `skip ${action.amount}`;
    case "distinctBy":
      return `distinct by ${action.field || "(value)"}`;
    default:
      return action.type;
  }
}

export function applyAction(data: unknown, action: TransformationAction): unknown {
  switch (action.type) {
    case "filterEquals": {
      const list = ensureArray(data, action.type);
      const expected = parseLooseLiteral(action.value);
      return list.filter((item) => compare(readPath(item, action.field), expected));
    }
    case "filterGreaterThan": {
      const list = ensureArray(data, action.type);
      return list.filter((item) => {
        const left = asNumber(readPath(item, action.field));
        return left !== null && left > action.amount;
      });
    }
    case "mapMultiply": {
      const list = ensureArray(data, action.type);
      const factor = action.amount;
      return list.map((item) => {
        if (!action.field) {
          const numeric = asNumber(item);
          return numeric === null ? item : numeric * factor;
        }

        const current = asNumber(readPath(item, action.field));
        if (current === null) {
          return item;
        }
        return setPath(item, action.field, current * factor);
      });
    }
    case "sortBy": {
      const list = ensureArray(data, action.type);
      const multiplier = action.direction === "desc" ? -1 : 1;
      return [...list].sort((a, b) => {
        const left = readPath(a, action.field);
        const right = readPath(b, action.field);
        if (left === right) return 0;
        if (left == null) return 1;
        if (right == null) return -1;
        return left > right ? 1 * multiplier : -1 * multiplier;
      });
    }
    case "groupBy": {
      const list = ensureArray(data, action.type);
      if (!action.field) {
        throw new Error("groupBy requires a field.");
      }
      return list.reduce<Record<string, unknown[]>>((acc, item) => {
        const key = String(readPath(item, action.field));
        const bucket = acc[key] ?? [];
        acc[key] = [...bucket, item];
        return acc;
      }, {});
    }
    case "take": {
      const list = ensureArray(data, action.type);
      return list.slice(0, Math.max(0, action.amount));
    }
    case "skip": {
      const list = ensureArray(data, action.type);
      return list.slice(Math.max(0, action.amount));
    }
    case "distinctBy": {
      const list = ensureArray(data, action.type);
      const seen = new Set<string>();
      return list.filter((item) => {
        const keyValue = readPath(item, action.field);
        const token = stableToken(keyValue);
        if (seen.has(token)) return false;
        seen.add(token);
        return true;
      });
    }
    default:
      return data;
  }
}

export function buildSnapshots(sourceData: unknown, actions: TransformationAction[]): Snapshot[] {
  const snapshots: Snapshot[] = [
    {
      label: "Input",
      description: describeData(sourceData),
      data: clone(sourceData)
    }
  ];

  let current: unknown = clone(sourceData);
  for (const action of actions) {
    current = applyAction(current, action);
    snapshots.push({
      label: action.type,
      description: `${describeAction(action)} | ${describeData(current)}`,
      data: clone(current)
    });
  }

  return snapshots;
}
