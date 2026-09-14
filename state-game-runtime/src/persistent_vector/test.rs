#[cfg(test)]
mod tests {
    use super::super::PersistentVector;
    use super::*;
    use std::sync::Arc;

    // 1. 초기 생성 및 빈 벡터 상태 검증
    #[test]
    fn test_new_and_empty() {
        let vec: PersistentVector<i32> = PersistentVector::new();
        assert_eq!(vec.len(), 0);
        assert!(vec.is_empty());
        assert_eq!(vec.height(), 0);
        assert_eq!(vec.first(), None);
        assert_eq!(vec.last(), None);
        assert_eq!(vec.get(0), None);
    }

    // 2. Push 및 영속성(Persistence, 구조적 공유) 검증
    #[test]
    fn test_push_and_persistence() {
        let v0: PersistentVector<i32> = PersistentVector::new();
        let v1 = v0.push(Arc::new(10));
        let v2 = v1.push(Arc::new(20));
        let v3 = v2.push(Arc::new(30));

        // 이전 버전의 벡터 데이터가 전혀 변경되지 않고 유지되는지 확인 (불변성 검증)
        assert_eq!(v0.len(), 0);
        assert_eq!(v1.len(), 1);
        assert_eq!(v2.len(), 2);
        assert_eq!(v3.len(), 3);

        assert_eq!(*v1.get(0).unwrap(), 10);

        assert_eq!(*v2.get(0).unwrap(), 10);
        assert_eq!(*v2.get(1).unwrap(), 20);

        assert_eq!(*v3.get(0).unwrap(), 10);
        assert_eq!(*v3.get(1).unwrap(), 20);
        assert_eq!(*v3.get(2).unwrap(), 30);

        assert_eq!(*v3.first().unwrap(), 10);
        assert_eq!(*v3.last().unwrap(), 30);
    }

    // 3. 많은 원소 삽입 시 트리 레벨 확장(NODE_CAPACITY=4 초과) 검증
    #[test]
    fn test_large_push_and_tree_expansion() {
        let mut vec = PersistentVector::new();
        let count = 200;

        for i in 0..count {
            vec = vec.push(Arc::new(i));
            assert_eq!(vec.len(), i + 1);
            assert_eq!(*vec.get(i).unwrap(), i);
        }

        assert_eq!(vec.len(), count);
        assert!(vec.height() > 0); // 트리 높이가 상향되었는지 확인

        for i in 0..count {
            assert_eq!(*vec.get(i).unwrap(), i);
        }
    }

    // 4. 특정 인덱스 값 수정(set) 및 클로저 기반 업데이트(update) 검증
    #[test]
    fn test_set_and_update() {
        let mut vec = PersistentVector::new();
        for i in 0..20 {
            vec = vec.push(Arc::new(i * 10));
        }

        // set 검증
        let vec_set = vec.set(5, 999).unwrap();
        assert_eq!(*vec.get(5).unwrap(), 50); // 원본은 그대로 유지 (50)
        assert_eq!(*vec_set.get(5).unwrap(), 999); // 새 벡터만 변경 (999)
        assert_eq!(vec_set.len(), 20);

        // 범위를 벗어난 인덱스 오류 처리
        assert!(vec.set(100, 500).is_err());

        // update 검증 (값 기반 계산 업데이트)
        let vec_updated = vec.update(5, |val| val + 1).unwrap();
        assert_eq!(*vec_updated.get(5).unwrap(), 51);
        assert!(vec.update(100, |val| val + 1).is_err());
    }

    // 5. Pop 기능 및 트리 축소 검증
    #[test]
    fn test_pop() {
        let mut vec = PersistentVector::new();
        for i in 0..15 {
            vec = vec.push(Arc::new(i));
        }

        let mut current = vec;
        for i in (0..15).rev() {
            assert_eq!(current.len(), i + 1);
            assert_eq!(*current.last().unwrap(), i);
            current = current.pop().unwrap();
        }

        assert_eq!(current.len(), 0);
        assert!(current.is_empty());
        assert!(current.pop().is_none()); // 빈 벡터 pop 시 None 반환
    }

    // 6. Truncate (길이 잘라내기) 및 Clear 검증
    #[test]
    fn test_truncate_and_clear() {
        let mut vec = PersistentVector::new();
        for i in 0..30 {
            vec = vec.push(Arc::new(i));
        }

        let truncated = vec.truncate(10);
        assert_eq!(truncated.len(), 10);
        assert_eq!(vec.len(), 30); // 원본은 유지

        for i in 0..10 {
            assert_eq!(*truncated.get(i).unwrap(), i);
        }

        let cleared = vec.clear();
        assert_eq!(cleared.len(), 0);
        assert!(cleared.is_empty());
    }

    // 7. Append, Extend, Insert, Remove 기능 검증
    #[test]
    fn test_structure_modifications() {
        let mut vec1 = PersistentVector::new();
        for i in 0..5 {
            vec1 = vec1.push(Arc::new(i));
        }

        let mut vec2 = PersistentVector::new();
        for i in 5..10 {
            vec2 = vec2.push(Arc::new(i));
        }

        // append & extend
        let appended = vec1.append(&vec2);
        assert_eq!(appended.len(), 10);
        for i in 0..10 {
            assert_eq!(*appended.get(i).unwrap(), i);
        }

        // insert
        let inserted = vec1.insert(2, 99); // [0, 1, 99, 2, 3, 4]
        assert_eq!(inserted.len(), 6);
        assert_eq!(*inserted.get(2).unwrap(), 99);
        assert_eq!(*inserted.get(3).unwrap(), 2);

        // remove
        let removed = inserted.remove(2); // 다시 99 제거 -> [0, 1, 2, 3, 4]
        assert_eq!(removed.len(), 5);
        assert_eq!(*removed.get(2).unwrap(), 2);
    }

    // 8. Iter 및 DoubleEndedIterator(양방향 순회) 검증
    #[test]
    fn test_iterators() {
        let mut vec = PersistentVector::new();
        for i in 0..25 {
            vec = vec.push(Arc::new(i));
        }

        // 순방향 순회
        let forward: Vec<i32> = vec.iter().map(|x| *x).collect();
        let expected: Vec<i32> = (0..25).collect();
        assert_eq!(forward, expected);

        // 역방향 순회 (DoubleEndedIterator)
        let backward: Vec<i32> = vec.iter().rev().map(|x| *x).collect();
        let expected_rev: Vec<i32> = (0..25).rev().collect();
        assert_eq!(backward, expected_rev);

        // IntoIterator (참조 기반)
        let into_iter_vals: Vec<i32> = (&vec).into_iter().map(|x| *x).collect();
        assert_eq!(into_iter_vals, expected);

        // ExactSizeIterator 남은 개수 검증
        let mut iter = vec.iter();
        assert_eq!(iter.len(), 25);
        iter.next();
        assert_eq!(iter.len(), 24);
    }

    // 9. 정렬 관련 API 세트 전체 검증
    #[test]
    fn test_sorting_functions() {
        let nums = vec![42, 12, 88, 3, 15, 27, 99, 1];
        let mut vec = PersistentVector::new();
        for &n in &nums {
            vec = vec.push(Arc::new(n));
        }

        // 1) 기본 오름차순 정렬
        let sorted: Vec<i32> = vec.sort().iter().map(|x| *x).collect();
        assert_eq!(sorted, vec![1, 3, 12, 15, 27, 42, 88, 99]);

        // 2) Unstable 오름차순 정렬
        let sorted_unstable: Vec<i32> = vec.sort_unstable().iter().map(|x| *x).collect();
        assert_eq!(sorted_unstable, vec![1, 3, 12, 15, 27, 42, 88, 99]);

        // 3) 커스텀 비교 함수 (내림차순 정렬)
        let sorted_by: Vec<i32> = vec.sort_by(|a, b| b.cmp(a)).iter().map(|x| *x).collect();
        assert_eq!(sorted_by, vec![99, 88, 42, 27, 15, 12, 3, 1]);

        // 4) 키 추출 함수 정렬 (10의 자리 기준 정렬)
        let sorted_key: Vec<i32> = vec.sort_by_key(|&x| x % 10).iter().map(|x| *x).collect();
        assert_eq!(*sorted_key.first().unwrap(), 1); // 1 % 10 = 1

        // 5) 캐시된 키 정렬 (문자열 변환 기준 정렬)
        let sorted_cached: Vec<i32> = vec
            .sort_by_cached_key(|&x| x.to_string())
            .iter()
            .map(|x| *x)
            .collect();
        assert_eq!(sorted_cached.len(), nums.len());
    }
}

#[cfg(test)]
mod stress_tests {
    use super::super::PersistentVector;
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_random_operations_against_vec() {
        let mut pv = PersistentVector::new();
        let mut vec = Vec::new();

        let mut seed = 0x12345678u64;

        fn rand(seed: &mut u64) -> usize {
            *seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);

            (*seed >> 32) as usize
        }

        for _ in 0..10000 {
            match rand(&mut seed) % 5 {
                // push
                0 => {
                    let value = rand(&mut seed) as i32;

                    pv = pv.push(Arc::new(value));
                    vec.push(value);
                }

                // set
                1 => {
                    if !vec.is_empty() {
                        let index = rand(&mut seed) % vec.len();
                        let value = rand(&mut seed) as i32;

                        pv = pv.set(index, value).unwrap();
                        vec[index] = value;
                    }
                }

                // pop
                2 => {
                    if !vec.is_empty() {
                        pv = pv.pop().unwrap();
                        vec.pop();
                    }
                }

                // get
                3 => {
                    if !vec.is_empty() {
                        let index = rand(&mut seed) % vec.len();

                        assert_eq!(*pv.get(index).unwrap(), vec[index]);
                    }
                }

                // iterator check
                4 => {
                    let collected: Vec<_> = pv.iter().map(|x| *x).collect();

                    assert_eq!(collected, vec);
                }

                _ => unreachable!(),
            }

            assert_eq!(pv.len(), vec.len());

            for i in 0..vec.len() {
                assert_eq!(*pv.get(i).unwrap(), vec[i]);
            }
        }
    }

    #[test]
    fn test_large_tree_integrity() {
        let mut pv = PersistentVector::new();

        let count = 100_000;

        for i in 0..count {
            pv = pv.push(Arc::new(i));
        }

        assert_eq!(pv.len(), count);

        for i in 0..count {
            assert_eq!(*pv.get(i).unwrap(), i);
        }

        let collected: Vec<_> = pv.iter().map(|x| *x).collect();

        assert_eq!(collected.len(), count);

        assert_eq!(collected[0], 0);

        assert_eq!(collected[count - 1], count - 1);
    }

    #[test]
    fn test_persistence_after_many_versions() {
        let mut versions = Vec::new();

        let mut current = PersistentVector::new();

        for i in 0..1000 {
            current = current.push(Arc::new(i));
            versions.push(current.clone());
        }

        for (index, version) in versions.iter().enumerate() {
            assert_eq!(version.len(), index + 1);

            assert_eq!(*version.last().unwrap(), index);
        }
    }

    #[test]
    fn test_set_does_not_mutate_previous_version() {
        let mut original = PersistentVector::new();

        for i in 0..100 {
            original = original.push(Arc::new(i));
        }

        let modified = original.set(50, 999).unwrap();

        assert_eq!(*original.get(50).unwrap(), 50);

        assert_eq!(*modified.get(50).unwrap(), 999);

        for i in 0..100 {
            if i != 50 {
                assert_eq!(*original.get(i).unwrap(), *modified.get(i).unwrap());
            }
        }
    }

    #[test]
    fn test_iterator_exact_size_and_double_end() {
        let mut pv = PersistentVector::new();

        for i in 0..100 {
            pv = pv.push(Arc::new(i));
        }

        let mut iter = pv.iter();

        assert_eq!(iter.len(), 100);

        assert_eq!(*iter.next().unwrap(), 0);

        assert_eq!(*iter.next_back().unwrap(), 99);

        assert_eq!(iter.len(), 98);

        let rest: Vec<_> = iter.map(|x| *x).collect();

        assert_eq!(rest.len(), 98);
    }

    #[test]
    fn test_sort_preserves_original() {
        let mut pv = PersistentVector::new();

        for i in [5, 1, 9, 3, 7] {
            pv = pv.push(Arc::new(i));
        }

        let sorted = pv.sort();

        assert_eq!(
            pv.iter().map(|x| *x).collect::<Vec<_>>(),
            vec![5, 1, 9, 3, 7]
        );

        assert_eq!(
            sorted.iter().map(|x| *x).collect::<Vec<_>>(),
            vec![1, 3, 5, 7, 9]
        );
    }
}
