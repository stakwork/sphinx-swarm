export interface Tribes {
  name: string;
  preview: string;
  logo?: string;
  created?: string;
  pricePerMessage: number;
  amountToStake?: number;
}

export const initialTribes: Tribes[] = [
  {
    name: "Testing Sphinx",
    pricePerMessage: 0,
    preview: "",
    logo: "",
  },
  {
    name: "Sphinx Dev",
    pricePerMessage: 2,
    preview: "",
    logo: "",
  },
  {
    name: "Planet Sphinx",
    pricePerMessage: 4,
    preview: "",
    logo: "",
  },
  {
    name: "Music",
    pricePerMessage: 2,
    preview: "",
    logo: "",
  },
  {
    name: "Sport",
    pricePerMessage: 3,
    preview: "",
    logo: "",
  },
];
