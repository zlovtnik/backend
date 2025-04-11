export type Organization = {
    id: string
    name: string
    description?: string
    createdAt: Date
    updatedAt: Date
}

export type CreateOrganizationInput = {
    name: string
    description?: string
}

export type UpdateOrganizationInput = {
    name?: string
    description?: string
} 