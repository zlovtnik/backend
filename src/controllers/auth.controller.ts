import { Request, Response } from 'express';
import { AuthService, RegisterUserInput, LoginUserInput } from '../services/auth.service';

const authService = new AuthService();

export class AuthController {
    async register(req: Request, res: Response) {
        try {
            const userData: RegisterUserInput = req.body;
            const user = await authService.register(userData);
            res.status(201).json(user);
        } catch (error: any) {
            res.status(400).json({ error: error.message });
        }
    }

    async login(req: Request, res: Response) {
        try {
            const loginData: LoginUserInput = req.body;
            const user = await authService.login(loginData);
            res.status(200).json(user);
        } catch (error: any) {
            res.status(401).json({ error: error.message });
        }
    }
} 