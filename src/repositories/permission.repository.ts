import prisma from '../database/index'
import { Permission, CreatePermissionInput, UpdatePermissionInput } from '../types/permission'

export const permissionRepository = {
    async create(data: CreatePermissionInput): Promise<Permission> {
        const result = await prisma.permission.create({
            data,
        });
        return {
            ...result,
            description: result.description || null
        };
    },

    async findById(id: string): Promise<Permission | null> {
        const result = await prisma.permission.findUnique({
            where: { id },
        });
        if (!result) return null;
        return {
            ...result,
            description: result.description || null
        };
    },

    async findByUser(userId: string): Promise<Permission[]> {
        const user = await prisma.user.findUnique({
            where: { id: userId },
            include: { permissions: true }
        });
        if (!user) return [];
        return user.permissions.map(permission => ({
            ...permission,
            description: permission.description || null
        }));
    },

    async findAll(): Promise<Permission[]> {
        const results = await prisma.permission.findMany();
        return results.map((result: Permission) => ({
            ...result,
            description: result.description || null
        }));
    },

    async update(id: string, data: UpdatePermissionInput): Promise<Permission> {
        const result = await prisma.permission.update({
            where: { id },
            data,
        });
        return {
            ...result,
            description: result.description || null
        };
    },

    async delete(id: string): Promise<Permission> {
        const result = await prisma.permission.delete({
            where: { id },
        });
        return {
            ...result,
            description: result.description || null
        };
    },
} 