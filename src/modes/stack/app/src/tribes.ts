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
    preview: "https://cache.sphinx.chat/?tribe=XuOp5B9kC3CcL52svtl_LJJJFbV1OTgnq7thtOjdKJMnOuETIw_hlLkVfonozVIwz5wADlya_i946GiKFZAgMto0cDuk",
    logo: "",
  },
  {
    name: "Sphinx Dev",
    pricePerMessage: 2,
    preview: "https://cache.sphinx.chat/?tribe=XuOp5B9kC3CcL52svtl_LJJJFbV1OTgnq7thtOjdKJMnOuETIw_hlLkVfonozVIwz5wADlya_i946GiKFZAgMto0cDuk",
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
    preview: "https://cache.sphinx.chat/?tribe=XuOp5B9kC3CcL52svtl_LJJJFbV1OTgnq7thtOjdKJMnOuETIw_hlLkVfonozVIwz5wADlya_i946GiKFZAgMto0cDuk",
    logo: "",
  },
];
