# @dgv/ui

Shared UI component library built with React, Tailwind CSS, and shadcn/ui.

## Components

### Button

A versatile button component with multiple variants and sizes.

```tsx
import { Button } from '@dgv/ui/button'

function MyComponent() {
  return (
    <Button onClick={() => console.log('clicked')}>
      Click me
    </Button>
  )
}
```

#### Variants
- `default` - Primary button style
- `secondary` - Secondary button style  
- `outline` - Outlined button
- `ghost` - Transparent button
- `link` - Link-styled button
- `destructive` - Destructive action button

#### Sizes
- `default` - Standard size
- `sm` - Small size
- `lg` - Large size
- `icon` - Square icon button

## Usage in Apps

1. Add the package as a dependency:
```json
{
  "dependencies": {
    "@dgv/ui": "workspace:*"
  }
}
```

2. Import the component:
```tsx
import { Button } from '@dgv/ui/button'
```

3. Import the styles in your main entry file:
```tsx
import '@dgv/ui/styles.css'
```

## Development

This package uses TypeScript and follows the workspace pattern. Components are built with:
- React 19
- Tailwind CSS v4
- Radix UI primitives
- class-variance-authority for variant management




