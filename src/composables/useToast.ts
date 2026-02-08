import { ref, readonly } from "vue";

export type ToastType = "success" | "error" | "info";

export interface ToastMessage {
    id: number;
    type: ToastType;
    content: string;
}

const toasts = ref<ToastMessage[]>([]);
let nextId = 1;

export function useToast() {
    function add(type: ToastType, content: string, duration = 3000) {
        const id = nextId++;
        const message: ToastMessage = { id, type, content };
        toasts.value.push(message);

        if (duration > 0) {
            setTimeout(() => {
                remove(id);
            }, duration);
        }
    }

    function remove(id: number) {
        const index = toasts.value.findIndex((t) => t.id === id);
        if (index !== -1) {
            toasts.value.splice(index, 1);
        }
    }

    function success(content: string, duration = 3000) {
        add("success", content, duration);
    }

    function error(content: string, duration = 4000) {
        add("error", content, duration);
    }

    function info(content: string, duration = 3000) {
        add("info", content, duration);
    }

    return {
        toasts: readonly(toasts),
        add,
        remove,
        success,
        error,
        info
    };
}
