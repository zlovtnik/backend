import { PrismaClient } from '@prisma/client';
import { User } from '../types/user';

const prisma = new PrismaClient();

export const authService = {
    async login(email: string): Promise<User | null> {
        const user = await prisma.user.findUnique({
            where: { email },
        });
        if (!user) return null;
        return user;
    },

    async register(data: { email: string; password: string; name: string; organizationId: string }): Promise<User> {
        const user = await prisma.user.create({
            data,
        });
        return user;
    },
}; 