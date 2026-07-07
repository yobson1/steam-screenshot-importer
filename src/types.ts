export interface MenuButton {
	name: string;
	href?: string;
	src: string;
	rotate: boolean;
	onclick?: () => void | Promise<void>;
}
