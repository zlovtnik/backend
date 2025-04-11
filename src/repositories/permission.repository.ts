import { PrismaClient } from '@prisma/client';
import { Permission, CreatePermissionInput, UpdatePermissionInput } from '../types/permission'

const prisma = new PrismaClient();

export const permissionRepository = {
    async create(data: CreatePermissionInput): Promise<Permission> {
        const result = await prisma.permission.create({
            data,
        });
        return {
            ...result,
            description: result.description || undefined
        };
    },

    async findById(id: string): Promise<Permission | null> {
        const result = await prisma.permission.findUnique({
            where: { id },
        });
        if (!result) return null;
        return {
            ...result,
            description: result.description || undefined
        };
    },

    async findAll(): Promise<Permission[]> {
        const results = await prisma.permission.findMany();
        return results.map((result: any) => ({
            ...result,
            description: result.description || undefined
        }));
    },

    async update(id: string, data: UpdatePermissionInput): Promise<Permission> {
        const result = await prisma.permission.update({
            where: { id },
            data,
        });
        return {
            ...result,
            description: result.description || undefined
        };
    },

    async delete(id: string): Promise<Permission> {
        const result = await prisma.permission.delete({
            where: { id },
        });
        return {
            ...result,
            description: result.description || undefined
        };
    },
} 