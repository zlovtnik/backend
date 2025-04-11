import { PrismaClient } from '@prisma/client';
import { User, CreateUserInput, UpdateUserInput } from '../types/user'

const prisma = new PrismaClient();

export const userRepository = {
    async create(data: CreateUserInput): Promise<User> {
        return prisma.user.create({
            data,
        })
    },

    async findById(id: string): Promise<User | null> {
        return prisma.user.findUnique({
            where: { id },
        })
    },

    async findByEmail(email: string): Promise<User | null> {
        return prisma.user.findUnique({
            where: { email },
        })
    },

    async findAll(): Promise<User[]> {
        return prisma.user.findMany()
    },

    async update(id: string, data: UpdateUserInput): Promise<User> {
        return prisma.user.update({
            where: { id },
            data,
        })
    },

    async delete(id: string): Promise<User> {
        return prisma.user.delete({
            where: { id },
        })
    },
} 