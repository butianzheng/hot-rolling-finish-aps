import { z } from 'zod';

export const DateString = z.string().min(1);
export const DateTimeString = z.string().min(1);

