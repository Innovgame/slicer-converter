import { cva, type VariantProps } from "class-variance-authority";
import { Slot } from "radix-ui";
import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

const badgeVariants = cva(
    "inline-flex h-5 shrink-0 items-center rounded-[4px] border px-1.5 font-mono text-[11px] font-medium whitespace-nowrap",
    {
        variants: {
            variant: {
                default: "border-primary/25 bg-primary/15 text-primary",
                secondary: "border-border bg-muted text-muted-foreground",
                outline: "border-border text-foreground"
            }
        },
        defaultVariants: {
            variant: "default"
        }
    }
);

/**
 * @zh 短标签。用于格式名和扩展名，不承载业务状态。
 * @en Short label for format names and extensions. It does not own business state.
 */
function Badge({
    className,
    variant = "default",
    asChild = false,
    ...props
}: ComponentProps<"span"> & VariantProps<typeof badgeVariants> & { asChild?: boolean }) {
    const Comp = asChild ? Slot.Root : "span";

    return <Comp data-slot="badge" className={cn(badgeVariants({ variant, className }))} {...props} />;
}

export { Badge, badgeVariants };
