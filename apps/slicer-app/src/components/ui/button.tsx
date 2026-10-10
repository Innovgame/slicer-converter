import { cva, type VariantProps } from "class-variance-authority";
import { Slot } from "radix-ui";
import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
    "inline-flex shrink-0 items-center justify-center gap-1.5 rounded-md border border-transparent text-xs font-medium whitespace-nowrap transition-[background-color,border-color,box-shadow,transform] duration-150 outline-none select-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background active:translate-y-px disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-3.5",
    {
        variants: {
            variant: {
                default: "border-primary bg-primary text-primary-foreground hover:bg-primary/85",
                outline: "border-border bg-background hover:border-outline-strong hover:bg-muted hover:text-foreground",
                secondary:
                    "border-border bg-secondary text-secondary-foreground hover:border-outline-strong hover:bg-muted",
                ghost: "hover:border-border hover:bg-muted hover:text-foreground",
                destructive: "border-destructive/40 bg-destructive/12 text-destructive hover:bg-destructive/20"
            },
            size: {
                default: "h-8 px-3",
                sm: "h-7 px-2.5",
                icon: "size-8 border-0 bg-transparent text-muted-foreground shadow-none hover:bg-muted hover:text-foreground",
                "icon-sm":
                    "size-7 border-0 bg-transparent text-muted-foreground shadow-none hover:bg-muted hover:text-foreground"
            }
        },
        defaultVariants: {
            variant: "default",
            size: "default"
        }
    }
);

/**
 * @zh 无业务状态的按钮。视觉来自 shadcn / Radix，不拥有文档状态。
 * @en Stateless button. Visuals come from shadcn / Radix and it does not own document state.
 */
function Button({
    className,
    variant = "default",
    size = "default",
    asChild = false,
    ...props
}: ComponentProps<"button"> &
    VariantProps<typeof buttonVariants> & {
        asChild?: boolean;
    }) {
    const Comp = asChild ? Slot.Root : "button";

    return (
        <Comp
            data-slot="button"
            data-variant={variant}
            data-size={size}
            className={cn(buttonVariants({ variant, size, className }))}
            {...props}
        />
    );
}

export { Button, buttonVariants };
