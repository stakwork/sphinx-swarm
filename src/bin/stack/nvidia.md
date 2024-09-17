# nvidia on debian

### install nvidia driver:

to see your graphics card:

`lspci -nn | egrep -i "3d|display|vga"`

NVIDIA Corporation TU104GL [Tesla T4] [10de:1eb8] (rev a1)

add non-free to debian

`sudo sed -i 's/^Components: main$/& contrib non-free non-free-firmware/' /etc/apt/sources.list.d/debian.sources`

`sudo apt update`

`sudo apt install nvidia-detect`

`nvidia-detect`

`sudo apt install linux-headers-$(uname -r)`

`sudo apt install nvidia-driver`

reboot

`nvidia-smi` to check its working

### install nvidia container toolkit

`sudo apt install gpg`

```
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg \
  && curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | \
    sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
    sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
```

`sudo apt-get update`

`sudo apt-get install -y nvidia-container-toolkit`

`sudo nvidia-ctk runtime configure --runtime=docker`

`sudo systemctl restart docker`
