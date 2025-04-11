import prisma from '../database'
import { Permission, CreatePermissionInput, UpdatePermissionInput } from '../types/permission'

export const permissionRepository = {
    async create(data: CreatePermissionInput): Promise<Permission> {
        return prisma.permission.create({
            data,
        })
    },

    async findById(id: string): Promise<Permission | null> {
        return prisma.permission.findUnique({
            where: { id },
        })
    },

    async findAll(): Promise<Permission[]> {
        return prisma.permission.findMany()
    },

    async update(id: string, data: UpdatePermissionInput): Promise<Permission> {
        return prisma.permission.update({
            where: { id },
            data,
        })
    },

    async delete(id: string): Promise<Permission> {
        return prisma.permission.delete({
            where: { id },
        })
    },
} 