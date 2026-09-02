set -eu

cat > composer.json <<'EOF'
{
    "require": {
        "acme/example": "1.0.0"
    }
}
EOF

mkdir -p src

cat > src/App.php <<'EOF'
<?php

echo "hello\n";
EOF
