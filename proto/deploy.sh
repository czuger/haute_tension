rsync -avz --progress \
  --exclude='__pycache__/' \
  --exclude='*.pyc' \
  --exclude='*.pyo' \
  --exclude='*.pyd' \
  --exclude='.Python' \
  --exclude='*.so' \
  --exclude='.pytest_cache/' \
  --exclude='*.egg-info/' \
  --exclude='build/' \
  --exclude='dist/' \
  ../docker ../proto rpi4fe:/mnt/hdd1/python/haute_tension/

#ssh rpi4fe "docker restart haute-tension"

ssh rpi4fe "sudo supervisorctl restart haute_tension"