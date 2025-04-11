import { PrismaClient } from '@prisma/client';
import { hash, compare } from 'bcrypt';

const prisma = new PrismaClient();

export interface RegisterUserInput {
    email: string;
    password: string;
    name: string;
    organizationId: string;
}

export interface LoginUserInput {
    email: string;
    password: string;
}

export class AuthService {
    async register(userData: RegisterUserInput) {
        try {
            // For now, we're not hashing the password as requested
            const user = await prisma.user.create({
                data: {
                    email: userData.email,
                    password: userData.password, // In a real app, this should be hashed
                    name: userData.name,
                    organizationId: userData.organizationId,
                },
            });

            return {
                id: user.id,
                email: user.email,
                name: user.name,
            };
        } catch (error) {
            throw new Error('Failed to create user');
        }
    }

    async login(loginData: LoginUserInput) {
        try {
            const user = await prisma.user.findUnique({
                where: { email: loginData.email },
            });

            if (!user) {
                throw new Error('User not found');
            }

            // For now, we're doing a direct comparison as requested
            if (user.password !== loginData.password) {
                throw new Error('Invalid password');
            }

            return {
                id: user.id,
                email: user.email,
                name: user.name,
            };
        } catch (error) {
            throw new Error('Login failed');
        }
    }
} 