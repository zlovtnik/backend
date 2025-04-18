import prisma from '../database/index'
import { User, CreateUserInput, UpdateUserInput } from '../types/user'

export const userRepository = {
    async create(data: CreateUserInput): Promise<User> {
        const result = await prisma.user.create({
            data,
        });
        return {
            ...result,
            password: result.password
        };
    },

    async findById(id: string): Promise<User | null> {
        const result = await prisma.user.findUnique({
            where: { id },
        });
        if (!result) return null;
        return {
            ...result,
            password: result.password
        };
    },

    async findByEmail(email: string): Promise<User | null> {
        const result = await prisma.user.findUnique({
            where: { email },
        });
        if (!result) return null;
        return {
            ...result,
            password: result.password
        };
    },

    async findByOrganization(organizationId: string): Promise<User[]> {
        const results = await prisma.user.findMany({
            where: { organizationId },
        });
        return results.map(result => ({
            ...result,
            password: result.password
        }));
    },

    async findAll(): Promise<User[]> {
        const results = await prisma.user.findMany();
        return results.map(result => ({
            ...result,
            password: result.password
        }));
    },

    async update(id: string, data: UpdateUserInput): Promise<User> {
        const result = await prisma.user.update({
            where: { id },
            data,
        });
        return {
            ...result,
            password: result.password
        };
    },

    async delete(id: string): Promise<User> {
        const result = await prisma.user.delete({
            where: { id },
        });
        return {
            ...result,
            password: result.password
        };
    },
} 