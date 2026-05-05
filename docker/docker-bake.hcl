group "default" {
  targets = ["kairo-ecs-cli"]
}

variable "VERSION" {
  default = "dev"
}

variable "GIT_SHA" {
  default = "local"
}

target "kairo-ecs-cli" {
  context = "."
  dockerfile = "docker/Dockerfile"
  platforms = ["linux/amd64", "linux/arm64"]
  tags = [
    "kairo-ecs-cli:${VERSION}",
    "kairo-ecs-cli:${VERSION}-${GIT_SHA}",
    "kairo-ecs-cli:latest",
  ]
}

target "kairo-ecs-cli-pr" {
  inherits = ["kairo-ecs-cli"]
  platforms = ["linux/amd64"]
  tags = ["kairo-ecs-cli:${GIT_SHA}"]
}
