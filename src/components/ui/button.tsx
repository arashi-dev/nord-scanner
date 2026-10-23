import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-lg text-sm font-medium transition-all duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 active:scale-[0.98]",
  {
    variants: {
      variant: {
        default:
          "bg-cyan-500 text-white shadow-md shadow-cyan-500/20 hover:bg-cyan-400 hover:shadow-cyan-500/30",
        destructive:
          "bg-rose-600 text-white shadow-sm hover:bg-rose-500 shadow-rose-600/20",
        outline:
          "border border-slate-700 bg-slate-900/60 text-slate-200 hover:bg-slate-800 hover:border-slate-600 hover:text-white",
        secondary:
          "bg-slate-800 text-slate-200 hover:bg-slate-700 hover:text-white",
        ghost:
          "text-slate-300 hover:bg-slate-800/80 hover:text-white",
        link: "text-cyan-400 underline-offset-4 hover:underline",
        emerald:
          "bg-emerald-600 text-white shadow-md shadow-emerald-600/20 hover:bg-emerald-500",
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 rounded-md px-3 text-xs",
        lg: "h-10 rounded-lg px-6 text-base",
        icon: "h-9 w-9",
        xs: "h-7 px-2.5 text-xs rounded",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => {
    return (
      <button
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  }
);
Button.displayName = "Button";

export { Button, buttonVariants };
