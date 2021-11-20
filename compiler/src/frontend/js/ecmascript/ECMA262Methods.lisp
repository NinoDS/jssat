;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
; ECMA262 IR FILE IMPLEMENTATION ;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;
; This file is an implementation of ECMA262 https://tc39.es/ecma262/
; as a JSSAT IR File.
;
; A JSSAT IR File can be briefly described as follows: a list of S-expressions,
; where:
;
; - a `(def ...)` S-expression represents a rule rewrite
; - a `(section ...)` S-expression represents an ECMAScript function
;
; Once a JSSAT IR File is parsed, it produces the list of S-expressions, with all
; rule rewrites applied. It is then used to generate JSSAT IR, which is compiled
; into the program.
;
; For more information on JSSAT IR Files, see the `ir_file` crate.
;

(def (and :a :b) (:a and :b))
(def (or :a :b) (:a or :b))
(def (and3 :a :b :c) (and (and :a :b) :c))
(def (and4 :a :b :c :d) (and3 :a :b (and :c :d)))
(def (and6 :1 :2 :3 :4 :5 :6) (and (and3 :1 :2 :3) (and3 :4 :5 :6)))
(def (or3 :1 :2 :3) (or (or :1 :2) :3))
(def (or7 :1 :2 :3 :4 :5 :6 :7) (or3 (or3 :1 :2 :3) (or3 :4 :5 :6) :7))
(def (both :a :b (:x :y)) (and (:a :x :y) (:b :x :y)))
(def (both :1 :2 :f) (and (:f :1) (:f :2)))
(def (either :a :b (:x :y)) (or (:a :x :y) (:b :x :y)))
(def (todo) (assert false "TODO"))
(def (:a - :b) (:a + (not :b)))
(def (throw :x) (return (ThrowCompletion :x)))
(def (ret-comp :x) (return (NormalCompletion :x)))

(def
  (elif :condition :then-expr :end-expr)
  ((if :condition :then-expr :end-expr)))

(def
  (if-elif3-else
   (:cond :then)
   (:cond1 :then1)
   (:cond2 :then2)
   :else)
  (if :cond
      :then
      (elif :cond1 :then1
            (elif :cond2 :then2 :else))))

; TODO: implement a real list pop
(def (exec-ctx-stack-pop) (get-global JSSATExecutionContextStack <- list-new))
(def exec-ctx-stack (get-global -> JSSATExecutionContextStack))
(def (exec-ctx-stack-push :x) (list-push exec-ctx-stack :x))
(def exec-ctx-stack-size (list-len exec-ctx-stack))
(def curr-exec-ctx (list-get exec-ctx-stack (list-end exec-ctx-stack)))
(def current-realm (curr-exec-ctx -> Realm))

(def for-item (list-get :jssat_list :jssat_i))
(def for-item-rev (list-get :jssat_list (:jssat_len - (:jssat_i + 1))))

(def
  (for :list :body)
  (loop
        ((jssat_list = :list) (jssat_i = 0) (jssat_len = (list-len :list)))
        (:jssat_i < :jssat_len)
        ((jssat_list = :jssat_list) (jssat_i = (:jssat_i + 1)) (jssat_len = :jssat_len))
        :body))

(def
  (list-push :list :x)
  (list-set :list (math-max (list-end :list) 0) :x))

(def (list-end :list) ((list-len :list) - 1))

(def
  (list-new-1 :1)
  (expr-block
   ((jssat_list_temp = list-new)
    (list-push :jssat_list_temp :1)
    (:jssat_list_temp))))

(def
  (list-new-2 :1 :2)
  (expr-block
   ((jssat_list_temp = list-new)
    (list-push :jssat_list_temp :1)
    (list-push :jssat_list_temp :2)
    (:jssat_list_temp))))

(def
  (list-concat :a :b)
  (expr-block
   ((for :b ((list-push :a for-item)))
    (:a))))

; i'm too lazy to change let exprs to expr blocks atm
(def (expr-block :x) (let
                       _discard
                       =
                       0
                       in
                       :x))

; only `not x`, `x == y`, and `x < y` are implemented. create the other operators here
(def (:x != :y) (not (:x == :y)))
(def (:x <= :y) ((:x == :y) or (:x < :y)))
(def (:x > :y) (:y < :x))
(def (:x >= :y) (:y <= :x))

(def String Bytes)
(def BigInt BigNumber)

(def null (trivial Null))
(def undefined (trivial Undefined))
(def normal (trivial Normal))
(def empty (trivial Empty))
(def unresolvable (trivial Unresolvable))
(def lexical-this (trivial LexicalThis))
(def lexical (trivial Lexical))
(def initialized (trivial Initialized))
(def uninitialized (trivial Uninitialized))
(def trivial-strict (trivial Strict))
(def trivial-global (trivial Global))
(def trivial-return (trivial Return))
(def (trivial throw) (trivial Throw))

(def (is-undef :x) (:x == undefined))
(def (isnt-undef :x) (:x != undefined))
(def (is-null :x) (:x == null))
(def (isnt-null :x) (:x != null))
(def (is-true :x) (:x == true))
(def (is-false :x) (:x == false))
(def (is-normal :x) (:x == normal))
(def (is-empty :x) (:x == empty))
(def (is-string :x) (is-type-of String :x))
(def (is-symbol :x) (is-type-of Symbol :x))
(def (is-number :x) (is-type-of Number :x))
(def (is-bigint :x) (is-type-of BigInt :x))
(def (is-bool :x) (is-type-of Boolean :x))
(def (is-record :x) (is-type-of Record :x))
(def (isnt-record :x) (not (is-record :x)))
(def (is-object :x) (is-record :x))

(def
  (match-pn :parseNode :kind :variant_idx)
  (and (:parseNode -> JSSATParseNodeKind == :kind) (:parseNode -> JSSATParseNodeVariant == :variant_idx)))

(def
  (is-pn :kind :variant_idx)
  (match-pn :parseNode (trivial-node :kind) :variant_idx))

(def (isnt-type-as :x :y) (not (is-type-as :x :y)))

(def
  (math-max :x :y)
  (expr-block
   ((if (:x > :y)
        ((:x))
        ((:y))))))

(def (:1 -> :2) (record-get-slot :1 :2))
(def (:1 -> :2 == :3) ((:1 -> :2) == :3))
(def (:1 -> :2 -> :3) ((:1 -> :2) -> :3))
(def (:1 => :2) (record-get-prop :1 :2))
(def (:record :slot <- :expr) (record-set-slot :record :slot :expr))
(def (:record :slot <-) (record-del-slot :record :slot))
(def (record-absent-slot :record :slot) (not (record-has-slot :record :slot)))
(def (record-absent-prop :record :prop) (not (record-has-prop :record :prop)))

(def
  (record-do-slot :bind :record :slot :action)
  (if (record-has-slot :record :slot)
      ((:bind = (record-get-slot :record :slot))
       :action)))

(def
  (record-copy-slot-or-default :src :dest :slot :default)
  (if (record-absent-slot :src :slot)
      ((record-set-slot :dest :slot :default))
      ((record-set-slot :dest :slot (record-get-slot :src :slot)))))

(def
  (record-copy-slot-if-present :src :dest :slot)
  (if (record-has-slot :src :slot)
      ((record-set-slot :dest :slot (:src -> :slot)))))

(def (record-absent-slot2 :r :1 :2)
  (and (record-absent-slot :r :1) (record-absent-slot :r :2)))

(def (record-absent-slot6 :r :s1 :s2 :s3 :s4 :s5 :s6)
  (and6 (record-absent-slot :r :s1) (record-absent-slot :r :s2)
        (record-absent-slot :r :s3) (record-absent-slot :r :s4)
        (record-absent-slot :r :s5) (record-absent-slot :r :s6)))

(def (isnt-abrupt-completion :x) (not (is-abrupt-completion :x)))
(def
  (is-abrupt-completion :x)
  (if (record-has-slot :x Type)
      (((record-get-slot :x Type) != normal))
      (false)))

(def
  (is-completion-record :x)
  (and3
   (record-has-slot :x Type)
   (record-has-slot :x Value)
   (record-has-slot :x Target)))

; TODO: somehow use `env` to load the `SyntaxError` object and construct it
(def (SyntaxError :env :msg) (:msg))
; same for TypeError (:env can be gotten via curr-exec-ctx)
(def (TypeError :msg) (:msg))
(def (ReferenceError :msg) (:msg))

; "Let <thing> be the sole element of <list>"
(def
  (sole-element :x)
  ((let
     jssat_list
     =
     :x
     in
     (; assert that the list is a list with a singular element
      (assert ((list-len :jssat_list) == 1) "to get the 'sole element' of a list, it must be a singleton list")
      (assert (list-has :jssat_list 1) "sanity check")
      (list-get :jssat_list 0)))))

; 5.2.3.4 ReturnIfAbrupt Shorthands
(def
  (! :OperationName)
  (expr-block
   (;;; 1. Let val be OperationName().
    (val = :OperationName)
    ; if we're not dealing with an object, it's already unwrapped
    (if (isnt-record :val)
        ((:val))
        (;;; 2. Assert: val is never an abrupt completion.
         (assert (isnt-abrupt-completion :val) "val is never an abrupt completion")
         ;;; 3. If val is a Completion Record, set val to val.[[Value]].
         (if (is-completion-record :val)
             ((record-get-slot :val Value))
             (:val)))))))

(def (Perform ! :x)
  (if (is-record :x)
      (;;; 2. Assert: val is never an abrupt completion.
       (assert (isnt-abrupt-completion :x) "val is never an abrupt completion"))))

; 5.2.3.4 ReturnIfAbrupt Shorthands
(def
  (? :x)
  (expr-block
   ((jssat_arg = :x)
    (; if we're not dealing with an object, it's already unwrapped
     (if (isnt-record :jssat_arg)
         ((:jssat_arg))
         (;;; 1. If argument is an abrupt completion, return argument.
          (if (is-abrupt-completion :jssat_arg)
              ((return :jssat_arg)
               (unreachable))
              ;;; 2. Else if argument is a Completion Record, set argument to argument.[[Value]].
              (elif (is-completion-record :jssat_arg)
                    ((record-get-slot :jssat_arg Value))
                    (:jssat_arg)))))))))

; 6.2.3.2  NormalCompletion
(def
  (NormalCompletion :x)
  (expr-block
   ((jssat_normal_completion = record-new)
    (:jssat_normal_completion Type <- normal)
    (:jssat_normal_completion Value <- :x)
    (:jssat_normal_completion Target <- empty)
    (:jssat_normal_completion))))

; 6.2.3.3  ThrowCompletion
(def
  (ThrowCompletion :x)
  (expr-block
   ((jssat_throw_completion = record-new)
    (:jssat_throw_completion Type <- (trivial throw))
    (:jssat_throw_completion Value <- :x)
    (:jssat_throw_completion Target <- empty)
    (:jssat_throw_completion))))

; "new declarative environment record"
(def new-declarative-environment-record
  (expr-block
   ((rec = record-new)
    (:rec JSSATHasBinding <- (get-fn-ptr DeclarativeEnvironmentRecord_HasBinding))
    (:rec))))

;;;;;;;
; virt calls
;;;;;;;

(def (virt0 :actor :slot) (call-virt (:actor -> :slot) :actor))
(def (virt1 :actor :slot :1) (call-virt (:actor -> :slot) :actor :1))
(def (virt2 :actor :slot :1 :2) (call-virt (:actor -> :slot) :actor :1 :2))

; we use `..` instead of `.` because `.` is a cons cell :v
(def (:env .. HasVarDeclaration :N) (call-virt (:env -> JSSATHasVarDeclaration) :env :N))
(def (:env .. HasLexicalDeclaration :N) (call-virt (:env -> JSSATHasLexicalDeclaration) :env :N))
(def (:env .. HasRestrictedGlobalProperty :N) (call-virt (:env -> JSSATHasRestrictedGlobalProperty) :env :N))
(def (:env .. HasBinding :N) (call-virt (:env -> JSSATHasBinding) :env :N))

; TODO: once we have all of these defined we should then replace them all with the single rule
; (def (:O .. :slot :P) (virt1 :O :slot :P))
; but for now we have each of these listed explicitly so we know how much we've done
(def (:O .. GetOwnProperty :P) (call-virt (:O -> GetOwnProperty) :O :P))
(def (:O .. GetPrototypeOf) (call-virt (:O -> GetPrototypeOf) :O))
(def (:O .. HasOwnProperty :P) (call-virt (:O -> HasOwnProperty) :O :P))
(def (:O .. HasProperty :P) (call-virt (:O -> HasProperty) :O :P))
(def (:O .. DefineOwnProperty :P :Desc) (call-virt (:O -> DefineOwnProperty) :O :P :Desc))
(def (:O .. IsExtensible) (virt0 :O IsExtensible))

(def (:func .. Call :thisValue :argumentList) (virt2 :func Call :thisValue :argmentList))

(def (evaluating :x) (call-virt (:x -> JSSATParseNodeEvaluate) :x))

; Table 34

(def (inject-table-34 :list)
  (_dontCare =
             (expr-block
              ((list-push :internalSlotsList (trivial-slot Environment))
               (list-push :internalSlotsList (trivial-slot PrivateEnvironment))
               (list-push :internalSlotsList (trivial-slot FormalParameters))
               (list-push :internalSlotsList (trivial-slot ECMAScriptCode))
               (list-push :internalSlotsList (trivial-slot ConstructorKind))
               (list-push :internalSlotsList (trivial-slot Realm))
               (list-push :internalSlotsList (trivial-slot ScriptOrModule))
               (list-push :internalSlotsList (trivial-slot ThisMode))
               (list-push :internalSlotsList (trivial-slot Strict))
               (list-push :internalSlotsList (trivial-slot HomeObject))
               (list-push :internalSlotsList (trivial-slot SourceText))
               (list-push :internalSlotsList (trivial-slot Fields))
               (list-push :internalSlotsList (trivial-slot PrivateMethods))
               (list-push :internalSlotsList (trivial-slot ClassFieldInitializerName))
               (list-push :internalSlotsList (trivial-slot IsClassConstructor))
               (undefined)))))

;;;;;;;;;;;;;;;;;;
; something ; (STATIC SEMANTICS AND RUNTIME SEMANTICS WIP SECTION)
;;;;;;;;;;;;;;;;;;
; well not really jssat behavior, more like implementation of static semantics
; and the way static semantics are is that there's a jssat record for each ast
; node with associated virtual methods that are implemented here
; wait but we have runtime semantics too h m mm

; TODO: these are all ParseNode related
; we need to declare the algorithm steps here, then link to them in the code that
; generates js objects from ecmascript spec

; helpers
(section
  (:0.0.0.0 InitializeJSSATThreadedGlobal ())
  ((get-global JSSATExecutionContextStack <- list-new)
   (return)))

;;;;;;;;;;;;;;;;;;;;;;;;;;
; METHOD IMPLEMENTATIONS ;
;;;;;;;;;;;;;;;;;;;;;;;;;;

(section
  (:6.1.6.1.14 Number::sameValue (x, y))
  ((return (:x == :y))))

(section
  (:6.1.6.2.14 BigInt::sameValue (x, y))
  ((return (:x == :y))))

(section
  (:6.2.5.1 IsAccessorDescriptor (Desc))
  (;;; 1. If Desc is undefined, return false.
   (if (is-undef :Desc)
       ((return false)))
   ;;; 2. If both Desc.[[Get]] and Desc.[[Set]] are absent, return false.
   (if (record-absent-slot2 :Desc Get Set)
       ((return false)))
   ;;; 3. Return true.
   (return true)))

(section
  (:6.2.5.2 IsDataDescriptor (Desc))
  (;;; 1. If Desc is undefined, return false.
   (if (is-undef :Desc)
       ((return false)))
   ;;; 2. If both Desc.[[Value]] and Desc.[[Writable]] are absent, return false.
   (if (record-absent-slot2 :Desc Value Writable)
       ((return false)))
   ;;; 3. Return true.
   (return true)))

(section
  (:6.2.5.3 IsGenericDescriptor (Desc))
  (;;; 1. If Desc is undefined, return false.
   (if (is-undef :Desc)
       ((return false)))
   ;;; 2. If IsAccessorDescriptor(Desc) and IsDataDescriptor(Desc) are both false, return true.
   (if (both (call IsAccessorDescriptor :Desc) (call IsDataDescriptor :Desc) (== false))
       ((return true)))
   ;;; 3. Return false.
   (return false)))

(section
  (:7.1.18 ToObject (argument))
  ((if (is-undef :argument)
       ((throw (TypeError "undefined -> object no worky"))))
   (if (is-null :argument)
       ((throw (TypeError "null -> object no worky"))))
   (if (is-bool :argument)
       ((wrapper = record-new)
        (:wrapper BooleanData <- :argument)
        (return :wrapper)))
   (if (is-number :argument)
       ((wrapper = record-new)
        (:wrapper NumberData <- :argument)
        (return :wrapper)))
   (if (is-string :argument)
       ((wrapper = record-new)
        (:wrapper StringData <- :argument)
        (return :wrapper)))
   (if (is-symbol :argument)
       ((wrapper = record-new)
        (:wrapper SymbolData <- :argument)
        (return :wrapper)))
   (if (is-bigint :argument)
       ((wrapper = record-new)
        (:wrapper BigIntData <- :argument)
        (return :wrapper)))
   (if (is-record :argument)
       ((return :argument)))
   (return unreachable)))

(section
  (:7.2.5 IsExtensible (O))
  (;;; 1. Return ? O.[[IsExtensible]]().
   (return (? (:O .. IsExtensible)))))

(section
  (:7.2.7 IsPropertyKey (argument))
  (;;; 1. If Type(argument) is String, return true.
   (if (is-string :argument)
       ((return true)))
   ;;; 2. If Type(argument) is Symbol, return true.
   (if (is-symbol :argument)
       ((return true)))
   ;;; 3. Return false.
   (return false)))

(section
  (:7.2.10 SameValue (x, y))
  (;;; 1. If Type(x) is different from Type(y), return false.
   (if (isnt-type-as :x :y)
       ((return false)))
   ;;; 2. If Type(x) is Number, then
   (if (is-number :x)
       (;;; a. Return ! Number::sameValue(x, y).
        (return (! (call Number::sameValue :x :y)))))
   ;;; 3. If Type(x) is BigInt, then
   (if (is-bigint :x)
       (;;; a. Return ! BigInt::sameValue(x, y).
        (return (! (call BigInt::sameValue :x :y)))))
   ;;; 4. Return ! SameValueNonNumeric(x, y).
   (return (! (call SameValueNonNumeric :x :y)))))

(section
  (:7.2.12 SameValueNonNumeric (x, y))
  (;;; 1. Assert: Type(x) is the same as Type(y).
   (assert (is-type-as :x :y) "Type(x) is the same as Type(y)")
   ;;; 2. If Type(x) is Undefined, return true.
   (if (is-undef :x) ((return true)))
   ;;; 3. If Type(x) is Null, return true.
   (if (is-null :x) ((return true)))
   ;;; 4. If Type(x) is String, then
   (if (is-string :x)
       ;;; a. If x and y are exactly the same sequence of code units (same length and same code units at corresponding
       ;;;    indices), return true; otherwise, return false.
       ((return (:x == :y))))
   ;;; 5. If Type(x) is Boolean, then
   (if (is-bool :x)
       (;;; a. If x and y are both true or both false, return true; otherwise, return false.
        (return (:x == :y))))
   ;;; 6. If Type(x) is Symbol, then
   (if (is-symbol :x)
       (;;; a. If x and y are both the same Symbol value, return true; otherwise, return false.
        (return (:x == :y))))
   ;;; 7. If x and y are the same Object value, return true. Otherwise, return false.
   (return (:x == :y))))

(section
  (:7.3.1 MakeBasicObject (internalSlotsList))
  (;;; 1. Let obj be a newly created object with an internal slot for each name in internalSlotsList.
   (obj = record-new)
   ;;; 2. Set obj's essential internal methods to the default ordinary object definitions specified in 10.1.
   (:obj GetPrototypeOf <- (get-fn-ptr OrdinaryObjectInternalMethods_GetPrototypeOf))
   (:obj IsExtensible <- (get-fn-ptr OrdinaryObjectInternalMethods_IsExtensible))
   (:obj GetOwnProperty <- (get-fn-ptr OrdinaryObjectInternalMethods_GetOwnProperty))
   (:obj HasProperty <- (get-fn-ptr OrdinaryObjectInternalMethods_HasProperty))
   (:obj DefineOwnProperty <- (get-fn-ptr OrdinaryObjectInternalMethods_DefineOwnProperty))
   ;;; 3. Assert: If the caller will not be overriding both obj's [[GetPrototypeOf]] and [[SetPrototypeOf]] essential internal
   ;;;    methods, then internalSlotsList contains [[Prototype]].
   ;;; 4. Assert: If the caller will not be overriding all of obj's [[SetPrototypeOf]], [[IsExtensible]], and [[PreventExtensions]]
   ;;;    essential internal methods, then internalSlotsList contains [[Extensible]].
   ;;; 5. If internalSlotsList contains [[Extensible]], set obj.[[Extensible]] to true.
   (if (true)
       ((:obj Extensible <- true)))
   ;;; 6. Return obj.
   (return :obj)))

(section
  (:7.3.5 CreateDataProperty (O, P, V))
  (;;; 1. Let newDesc be the PropertyDescriptor { [[Value]]: V, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: true }.
   (newDesc = record-new)
   (:newDesc Value <- :V)
   (:newDesc Writable <- true)
   (:newDesc Enumerable <- true)
   (:newDesc Configurable <- true)
   ;;; 2. Return ? O.[[DefineOwnProperty]](P, newDesc).
   (return (? (:O .. DefineOwnProperty :P :newDesc)))))

(section
  (:7.3.9 DefinePropertyOrThrow (O, P, desc))
  (;;; 1. Let success be ? O.[[DefineOwnProperty]](P, desc).
   (success = (? (:O .. DefineOwnProperty :P :desc)))
   ;;; 2. If success is false, throw a TypeError exception.
   (if (is-false :success)
       ((throw (TypeError "can't define property"))))
   ;;; 3. Return success.
   (return :success)))

(section
  (:7.3.12 HasProperty (O, P))
  (;;; 1. Return ? O.[[HasProperty]](P).
   (return (? (:O .. HasProperty :P)))))

(section
  (:8.1.1 BoundNames (parseNode))
  (; BindingIdentifier : Identifier
   (if (match-pn :parseNode (trivial-node BindingIdentifier) 0)
       (;;; 1. Return a List whose sole element is the StringValue of Identifier.
        (return (list-new-1 (:parseNode -> JSSATParseNodeSlot1 -> JSSATParseNode_Identifier_StringValue)))))
   ; BindingIdentifier : yield
   (if (match-pn :parseNode (trivial-node BindingIdentifier) 1)
       (;;; 1. Return a List whose sole element "yield".
        (return (list-new-1 "yield"))))
   ; BindingIdentifier : await
   (if (match-pn :parseNode (trivial-node BindingIdentifier) 2)
       (;;; 1. Return a List whose sole element "await".
        (return (list-new-1 "await"))))
   (return list-new)))

(section
  (:8.1.4 LexicallyDeclaredNames (parseNode))
  ((return list-new)))

(section
  (:8.1.5 LexicallyScopedDeclarations (parseNode))
  ((return list-new)))

(section
  (:8.1.6 VarDeclaredNames (parseNode))
  ((return list-new)))

(section
  (:8.1.7 VarScopedDeclarations (parseNode))
  (; VarScopedDeclarations is defined to "Return a new empty List" for so many
   ; productions, that productions that do so are omitted.
   ;
   ; StatementList : StatementList StatementListItem
   (if (match-pn :parseNode (trivial-node StatementList) 1)
       (;;; 1. Let declarations1 be VarScopedDeclarations of StatementList.
        (declarations1 = (call VarScopedDeclarations (:parseNode -> JSSATParseNodeSlot1)))
        ;;; 2. Let declarations2 be VarScopedDeclarations of StatementListItem.
        (declarations2 = (call VarScopedDeclarations (:parseNode -> JSSATParseNodeSlot2)))
        ;;; 3. Return the list-concatenation of declarations1 and declarations2.
        (return (list-concat :declarations1 :declarations2))))
   ; VariableDeclarationList : VariableDeclaration
   (if (match-pn :parseNode (trivial-node VariableDeclarationList) 0)
       ((return (list-new-1 (:parseNode -> JSSATParseNodeSlot1)))))
   ; VariableDeclarationList : VariableDeclarationList , VariableDeclaration
   (if (is-pn VariableDeclarationList 1)
       (;;; 1. Let declarations1 be VarScopedDeclarations of VariableDeclarationList.
        (declarations1 = (call VarScopedDeclarations (:parseNode -> JSSATParseNodeSlot1)))
        ;;; 2. Return the list-concatenation of declarations1 and « VariableDeclaration ».
        (return (list-concat :declarations1 (list-new-1 (:parseNode -> JSSATParseNodeSlot2))))))
   ; ScriptBody : StatementList
   (if (is-pn ScriptBody 0)
       (;;; 1. Return TopLevelVarScopedDeclarations of StatementList.
        (return (call TopLevelVarScopedDeclarations (:parseNode -> JSSATParseNodeSlot1)))))
   (return list-new)))

(section
  (:8.1.8 TopLevelLexicallyDeclaredNames (parseNode))
  ((return list-new)))

(section
  (:8.1.11 TopLevelVarScopedDeclarations (parseNode))
  (; StatementList : StatementList StatementListItem
   (if (is-pn StatementList 1)
       (;;; 1. Let declarations1 be TopLevelVarScopedDeclarations of StatementList.
        (declarations1 = (call VarScopedDeclarations (:parseNode -> JSSATParseNodeSlot1)))
        ;;; 2. Let declarations2 be TopLevelVarScopedDeclarations of StatementListItem.
        (declarations2 = (call VarScopedDeclarations (:parseNode -> JSSATParseNodeSlot2)))
        ;;; 3. Return the list-concatenation of declarations1 and declarations2.
        (return (list-concat :declarations1 :declarations2))))
   ; StatementListItem : Statement
   (if (is-pn StatementListItem 0)
       (;;; 1. If Statement is Statement : LabelledStatement , return TopLevelVarScopedDeclarations of Statement.
        ;;; 2. Return VarScopedDeclarations of Statement.
        (return (call VarScopedDeclarations (:parseNode -> JSSATParseNodeSlot1)))))
   (return list-new)))

(section
  (:9.1.1.1.1 DeclarativeEnvironmentRecord_HasBinding (envRec, N))
  (;;; 1. If envRec has a binding for the name that is the value of N, return true.
   (if (record-has-prop :envRec :N)
       ((return true)))
   ;;; 2. Return false.
   (return false)))

(section
  (:9.1.1.2.1 ObjectEnvironmentRecord_HasBinding (envRec, N))
  (;;; 1. Let bindingObject be envRec.[[BindingObject]].
   (bindingObject = (:envRec -> BindingObject))
   ;;; 2. Let foundBinding be ? HasProperty(bindingObject, N).
   (foundBinding = (? (call HasProperty :bindingObject :N)))
   ;;; 3. If foundBinding is false, return false.
   (if (is-false :foundBinding)
       ((return false)))
   ;;; 4. If envRec.[[IsWithEnvironment]] is false, return true.
   (if (is-false (:envRec -> IsWithEnvironment))
       ((return true)))
   ;;; 5. Let unscopables be ? Get(bindingObject, @@unscopables).
   ;;; 6. If Type(unscopables) is Object, then
   ;;; a. Let blocked be ! ToBoolean(? Get(unscopables, N)).
   ;;; b. If blocked is true, return false.
   ;;; 7. Return true.
   (return true)))

(section
  (:9.1.1.3.1 BindThisValue (envRec, V))
  (;;; 1. Assert: envRec.[[ThisBindingStatus]] is not lexical.
   (assert ((:envRec -> ThisBindingStatus) != lexical) "envRec.[[ThisBindingStatus]] is not lexical.")
   ;;; 2. If envRec.[[ThisBindingStatus]] is initialized, throw a ReferenceError exception.
   (if ((:envRec -> ThisBindingStatus) == initialized)
       ((throw (ReferenceError "couldnt bind this value idk"))))
   ;;; 3. Set envRec.[[ThisValue]] to V.
   (:envRec ThisValue <- :V)
   ;;; 4. Set envRec.[[ThisBindingStatus]] to initialized.
   (:envRec ThisBindingStatus <- initialized)
   ;;; 5. Return V.
   (return :V)))

(section
  (:9.1.1.4.1 GlobalEnvironmentRecord_HasBinding (envRec, N))
  (;;; 1. Let DclRec be envRec.[[DeclarativeRecord]].
   (DclRec = (:envRec -> DeclarativeRecord))
   ;;; 2. If DclRec.HasBinding(N) is true, return true.
   (if (is-true (:DclRec .. HasBinding :N))
       ((return true)))
   ;;; 3. Let ObjRec be envRec.[[ObjectRecord]].
   (ObjRec = (:envRec -> ObjectRecord))
   ;;; 4. Return ? ObjRec.HasBinding(N).
   (return (? (:ObjRec .. HasBinding :N)))))

(section
  (:9.1.2.1 GetIdentifierReference (env, name, strict))
  (;;; 1. If env is the value null, then
   (if (is-null :env)
       (;;; a. Return the Reference Record { [[Base]]: unresolvable, [[ReferencedName]]: name, [[Strict]]: strict, [[ThisValue]]: empty }.
        (refRec = record-new)
        (:refRec Base <- unresolvable)
        (:refRec ReferencedName <- :name)
        (:refRec Strict <- :strict)
        (:refRec ThisValue <- empty)
        (return :refRec)))
   ;;; 2. Let exists be ? env.HasBinding(name).
   (exists = (? (:env .. HasBinding :name)))
   ;;; 3. If exists is true, then
   (if (is-true :exists)
       (;;; a. Return the Reference Record { [[Base]]: env, [[ReferencedName]]: name, [[Strict]]: strict, [[ThisValue]]: empty }.
        (refRec = record-new)
        (:refRec Base <- :env)
        (:refRec ReferencedName <- :name)
        (:refRec Strict <- :strict)
        (:refRec ThisValue <- empty)
        (return :refRec))
       ;;; 4. Else,
       (;;; a. Let outer be env.[[OuterEnv]].
        (outer = (:env -> OuterEnv))
        ;;; b. Return ? GetIdentifierReference(outer, name, strict).
        (return (? (call GetIdentifierReference :outer :name :strict)))))
   (return unreachable)))

(section
  (:9.1.2.3 NewObjectEnvironment (O, W, E))
  (;;; 1. Let env be a new object Environment Record.
   (env = record-new)
   (:env JSSATHasBinding <- (get-fn-ptr ObjectEnvironmentRecord_HasBinding))
   ;;; 2. Set env.[[BindingObject]] to O.
   (:env BindingObject <- :O)
   ;;; 3. Set env.[[IsWithEnvironment]] to W.
   (:env IsWithEnvironment <- :W)
   ;;; 4. Set env.[[OuterEnv]] to E.
   (:env OuterEnv <- :E)
   ;;; 5. Return env.
   (return :env)))

(section
  (:9.1.2.4 NewFunctionEnvironment (F, newTarget))
  (;;; 1. Let env be a new function Environment Record containing no bindings.
   (env = record-new)
   ;;; 2. Set env.[[FunctionObject]] to F.
   (:env FunctionObject <- :F)
   ;;; 3. If F.[[ThisMode]] is lexical, set env.[[ThisBindingStatus]] to lexical.
   (if ((:F -> ThisMode) == lexical)
       ((:env ThisBindingStatus <- lexical))
       (;;; 4. Else, set env.[[ThisBindingStatus]] to uninitialized.
        (:env ThisBindingStatus <- uninitialized)))
   ;;; 5. Set env.[[NewTarget]] to newTarget.
   (:env NewTarget <- :newTarget)
   ;;; 6. Set env.[[OuterEnv]] to F.[[Environment]].
   (:env OuterEnv <- (:F -> Environment))
   ;;; 7. Return env.
   (return :env)))

(section
  (:9.1.2.5 NewGlobalEnvironment (G, thisValue))
  (;;; 1. Let objRec be NewObjectEnvironment(G, false, null).
   (objRec = (call NewObjectEnvironment :G false null))
   ;;; 2. Let dclRec be a new declarative Environment Record containing no bindings.
   (dclRec = new-declarative-environment-record)
   ;;; 3. Let env be a new global Environment Record.
   (env = record-new)
   (:env JSSATHasBinding <- (get-fn-ptr GlobalEnvironmentRecord_HasBinding))
   ;;; 4. Set env.[[ObjectRecord]] to objRec.
   (:env ObjectRecord <- :objRec)
   ;;; 5. Set env.[[GlobalThisValue]] to thisValue.
   (:env GlobalThisValue <- :thisValue)
   ;;; 6. Set env.[[DeclarativeRecord]] to dclRec.
   (:env DeclarativeRecord <- :dclRec)
   ;;; 7. Set env.[[VarNames]] to a new empty List.
   (:env VarNames <- list-new)
   ;;; 8. Set env.[[OuterEnv]] to null.
   (:env OuterEnv <- null)
   ;;; 9. Return env.
   (return :env)))

(section
  (:9.3.1 CreateRealm ())
  (;;; 1. Let realmRec be a new Realm Record.
   (realmRec = record-new)
   ;;; 2. Perform CreateIntrinsics(realmRec).
   (call CreateIntrinsics :realmRec)
   ;;; 3. Set realmRec.[[GlobalObject]] to undefined.
   (:realmRec GlobalObject <- undefined)
   ;;; 4. Set realmRec.[[GlobalEnv]] to undefined.
   (:realmRec GlobalEnv <- undefined)
   ;;; 5. Set realmRec.[[TemplateMap]] to a new empty List.
   (:realmRec TemplateMap <- list-new)
   ;;; 6. Return realmRec.
   (return :realmRec)))

(section
  (:9.3.2 CreateIntrinsics (realmRec))
  (;;; 1. Let intrinsics be a new Record.
   (intrinsics = record-new)
   ;;; 2. Set realmRec.[[Intrinsics]] to intrinsics.
   (:realmRec Intrinsics <- :intrinsics)
   ;;; 3. Set fields of intrinsics with the values listed in Table 8. The field names are the names listed in column one
   ;;;    of the table. The value of each field is a new object value fully and recursively populated with property values
   ;;;    as defined by the specification of each object in clauses 19 through 28. All object property values are newly
   ;;;    created object values. All values that are built-in function objects are created by performing CreateBuiltinFunction(steps, length, name, slots, realmRec, prototype) where steps is the definition of that function provided by this specification, name is the initial value of the function's name property, length is the initial value of the function's length property, slots is a list of the names, if any, of the function's specified internal slots, and prototype is the specified value of the function's [[Prototype]] internal slot. The creation of the intrinsics and their properties must be ordered to avoid any dependencies upon objects that have not yet been created.
   ;;; 4. Perform AddRestrictedFunctionProperties(intrinsics.[[%Function.prototype%]], realmRec).
   ;;; 5. Return intrinsics.
   (return :intrinsics)))

(section
  (:9.3.3 SetRealmGlobalObject (realmRec, globalObj, thisValue))
  (;;; 1. If globalObj is undefined, then
   (globalObj =
              (expr-block
               ((if (is-undef :globalObj)
                    (;;; a. Let intrinsics be realmRec.[[Intrinsics]].
                     (intrinsics = (:realmRec -> Intrinsics))
                     ;;; b. Set globalObj to ! OrdinaryObjectCreate(intrinsics.[[%Object.prototype%]]).
                     ; TODO: actually use the intrinsics
                     (tmp = record-new)
                     (:tmp Prototype <- null)
                     (:tmp GetPrototypeOf <- (get-fn-ptr OrdinaryObjectInternalMethods_GetPrototypeOf))
                     (:tmp GetOwnProperty <- (get-fn-ptr OrdinaryObjectInternalMethods_GetOwnProperty))
                     (:tmp HasProperty <- (get-fn-ptr OrdinaryObjectInternalMethods_HasProperty))
                     (! (call OrdinaryObjectCreate :tmp list-new)))
                    ((:globalObj))))))
   ;;; 2. Assert: Type(globalObj) is Object.
   (assert (is-object :globalObj) "Type(globalObj) is Object")
   ;;; 3. If thisValue is undefined, set thisValue to globalObj.
   (thisValue =
              (expr-block
               ((if (is-undef :thisValue)
                    ((:globalObj))
                    ((:thisValue))))))
   ;;; 4. Set realmRec.[[GlobalObject]] to globalObj.
   (:realmRec GlobalObject <- :globalObj)
   ;;; 5. Let newGlobalEnv be NewGlobalEnvironment(globalObj, thisValue).
   (newGlobalEnv = (call NewGlobalEnvironment :globalObj :thisValue))
   ;;; 6. Set realmRec.[[GlobalEnv]] to newGlobalEnv.
   (:realmRec GlobalEnv <- :newGlobalEnv)
   ;;; 7. Return realmRec.
   (return :realmRec)))

(section
  (:9.3.4 SetDefaultGlobalBindings (realmRec))
  (;;; 1. Let global be realmRec.[[GlobalObject]].
   (global = (:realmRec -> GlobalObject))
   ;;; 2. For each property of the Global Object specified in clause 19, do
   ;;; a. Let name be the String value of the property name.
   ;;; b. Let desc be the fully populated data Property Descriptor for the property, containing the specified attributes for the property. For properties listed in 19.2, 19.3, or 19.4 the value of the [[Value]] attribute is the corresponding intrinsic object from realmRec.
   ;;; c. Perform ? DefinePropertyOrThrow(global, name, desc).
   ;;; 3. Return global.
   (return :global)))

(section
  (:9.4.1 GetActiveScriptOrModule ())
  (;;; 1. If the execution context stack is empty, return null.
   (if (exec-ctx-stack-size == 0)
       ((return null)))
   ;;; 2. Let ec be the topmost execution context on the execution context stack whose ScriptOrModule component is not null.
   ;;; 3. If no such execution context exists, return null. Otherwise, return ec's ScriptOrModule.
   (for exec-ctx-stack
        ((execCtx = for-item-rev)
         (scriptOrModule = (:execCtx -> ScriptOrModule))
         (if (isnt-null :scriptOrModule)
             ((return :scriptOrModule)))))
   (return null)))

(section
  (:9.4.2 ResolveBinding (name, env))
  (;;; 1. If env is not present or if env is undefined, then
   (env = (expr-block
           ((if (is-undef :env)
                (;;; a. Set env to the running execution context's LexicalEnvironment.
                 (curr-exec-ctx -> LexicalEnvironment))
                (:env)))))
   ;;; 2. Assert: env is an Environment Record.
   ;;; 3. If the source text matched by the syntactic production that is being evaluated is contained in strict mode code,
   ;;;    let strict be true; else let strict be false.
   (strict = true) ; TODO: revisit this
   ;;; 4. Return ? GetIdentifierReference(env, name, strict).
   (return (? (call GetIdentifierReference :env :name :strict)))))

(section
  (:9.5 InitializeHostDefinedRealm ())
  (;;; 1. Let realm be CreateRealm().
   (realm = (call CreateRealm))
   ;;; 2. Let newContext be a new execution context.
   (newContext = record-new)
   ; TODO: have a subroutine to make new execution context properly
   (:newContext LexicalEnvironment <- record-new)
   ;;; 3. Set the Function of newContext to null.
   (:newContext Function <- null)
   ;;; 4. Set the Realm of newContext to realm.
   (:newContext Realm <- :realm)
   ;;; 5. Set the ScriptOrModule of newContext to null.
   (:newContext ScriptOrModule <- null)
   ;;; 6. Push newContext onto the execution context stack; newContext is now the running execution context.
   (exec-ctx-stack-push :newContext)
   ;;; 7. If the host requires use of an exotic object to serve as realm's global object, let global be such an object created
   ;;;    in a host-defined manner. Otherwise, let global be undefined, indicating that an ordinary object should be created
   ;;;    as the global object.
   (global = undefined)
   ;;; 8. If the host requires that the this binding in realm's global scope return an object other than the global object,
   ;;;    let thisValue be such an object created in a host-defined manner. Otherwise, let thisValue be undefined, indicating
   ;;;    that realm's global this binding should be the global object.
   (thisValue = undefined)
   ;;; 9. Perform SetRealmGlobalObject(realm, global, thisValue).
   (call SetRealmGlobalObject :realm :global :thisValue)
   ;;; 10. Let globalObj be ? SetDefaultGlobalBindings(realm).
   (globalObj = (? (call SetDefaultGlobalBindings :realm)))
   ;;; 11. Create any host-defined global object properties on globalObj.
   ;;; 12. Return NormalCompletion(empty).
   (return (NormalCompletion empty))))

(section
  (:10.1.6.3 ValidateAndApplyPropertyDescriptor (O, P, extensible, Desc, current))
  (;;; 1. Assert: If O is not undefined, then IsPropertyKey(P) is true
   (if (isnt-undef :O)
       ((assert ((call IsPropertyKey :P) == true) "If O is not undefined, then IsPropertyKey(P) is true")))
   ;;; 2. If current is undefined, then
   (if (is-undef :current)
       (;;; a. If extensible is false, return false
        (if (:extensible == false)
            ((return false)))
        ;;; b. Assert: extensible is true.
        (assert (:extensible == true) "extensible is true")
        ;;; c. If IsGenericDescriptor(Desc) is true or IsDataDescriptor(Desc) is true, then
        (if (either (call IsGenericDescriptor :Desc) (call IsDataDescriptor :Desc) (== true))
            (;;; i. If O is not undefined, create an own data property named P of object O whose [[Value]],
             ;;;[[Writable]], [[Enumerable]], and [[Configurable]] attribute values are described by Desc. If the
             ;;;value of an attribute field of Desc is absent, the attribute of the newly created property is set to its
             ;;;default value.
             (if (isnt-undef :O)
                 ((p-desc = record-new)
                  (record-copy-slot-or-default :Desc :p-desc Value undefined)
                  (record-copy-slot-or-default :Desc :p-desc Writable undefined)
                  (record-copy-slot-or-default :Desc :p-desc Enumerable undefined)
                  (record-copy-slot-or-default :Desc :p-desc Configurable undefined)
                  (record-set-prop :O :P :p-desc))
                 ;;; d. Else,
                 (;; i. Assert: ! IsAccessorDescriptor(Desc) is true.
                  (assert (! (call IsAccessorDescriptor :Desc)) "! IsAccessorDescriptor(Desc) is true")
                  ;; ii. If O is not undefined, create an own accessor property named P of object O 
                  ;;     whose [[Get]], [[Set]], [[Enumerable]], and [[Configurable]] attribute values are described by
                  ;;     Desc. If the value of an attribute field of Desc is absent, the attribute of the newly created
                  ;;     property is set to its default value.
                  (if (isnt-undef :O)
                      ((p-desc = record-new)
                       (record-copy-slot-or-default :Desc :p-desc Get undefined)
                       (record-copy-slot-or-default :Desc :p-desc Set undefined)
                       (record-copy-slot-or-default :Desc :p-desc Enumerable undefined)
                       (record-copy-slot-or-default :Desc :p-desc Configurable undefined)
                       (record-set-prop :O :P :p-desc)))))
             ;;; e. Return true.
             (return true)))))
   ;;; 3. If every field in Desc is absent, return true.
   (if (record-absent-slot6 :Desc Value Writable Get Set Enumerable Configurable)
       ((return true)))
   ;;; 4. If current.[[Configurable]] is false, then
   (if (is-false (:current -> Configurable))
       (;;; a. If Desc.[[Configurable]] is present and its value is true, return false.
        (record-do-slot value :Desc Configurable
                        (if (is-true :value) ((return false))))
        ;;; b. If Desc.[[Enumerable]] is present and ! SameValue(Desc.[[Enumerable]], current.[[Enumerable]]) is false, return false.
        (if (record-has-slot :Desc Enumerable)
            ((if (is-false (! (call SameValue (:Desc -> Enumerable) (:current -> Enumerable))))
                 ((return false)))))))
   ;;; 5. If ! IsGenericDescriptor(Desc) is true, then
   (if-elif3-else
    ((is-true (! (call IsGenericDescriptor :Desc)))
     (;;; a. NOTE: No further validation is required.
     ))
    ;;; 6. Else if ! SameValue(! IsDataDescriptor(current), ! IsDataDescriptor(Desc)) is false, then
    ((is-false (! (call SameValue (! (call IsDataDescriptor :current)) (! (call IsDataDescriptor :Desc)))))
     (;;; a. If current.[[Configurable]] is false, return false.
      (if (is-false (:current -> Configurable)) ((return false)))
      ;;; b. If IsDataDescriptor(current) is true, then
      (if (is-true (call IsDataDescriptor :current))
          (;;; i. If O is not undefined, convert the property named P of object O from a data property to an
           ;;;    accessor property. Preserve the existing values of the converted property's [[Configurable]]
           ;;;    and [[Enumerable]] attributes and set the rest of the property's attributes to their default
           ;;;    values.
           (if (isnt-undef :O)
               ((P = (:O => :P))
                (:P Get <- undefined)
                (:P Set <- undefined)
                (:P Value <-)
                (:P Writable <-))))
          ;;; c. Else,
          (;;; i. If O is not undefined, convert the property named P of object O from an accessor property to
           ;;;    a data property. Preserve the existing values of the converted property's [[Configurable]]
           ;;;    and [[Enumerable]] attributes and set the rest of the property's attributes to their default
           ;;;    values.
           (if (isnt-undef :O)
               ((P = (:O => :P))
                (:P Value <- undefined)
                (:P Writable <- undefined)
                (:P Get <-)
                (:P Set <-)))))))
    ;;; 7. Else if IsDataDescriptor(current) and IsDataDescriptor(Desc) are both true, then
    ((both (call IsDataDescriptor :current) (call IsDataDescriptor :Desc) is-true)
     (;;; a. If current.[[Configurable]] is false and current.[[Writable]] is false, then
      (if (both (:current -> Configurable) (:current -> Writable) is-false)
          (;;; i. If Desc.[[Writable]] is present and Desc.[[Writable]] is true, return false.
           (record-do-slot writable :Desc Writable (if (is-true :writable) ((return true))))
           ;;; ii. If Desc.[[Value]] is present and SameValue(Desc.[[Value]], current.[[Value]]) is false, return false.
           (if (record-has-slot :Desc Value)
               ((if (is-false (call SameValue (:Desc -> Value) (:current -> Value)))
                    ((return false)))))
           ;;; iii. Return true.
           (return true)))))
    ;;;  8. Else,
    (;;; a. Assert: ! IsAccessorDescriptor(current) and ! IsAccessorDescriptor(Desc) are both true.
     (assert
      (both (! (call IsAccessorDescriptor :current)) (! (call IsAccessorDescriptor :Desc)) is-true)
      "! IsAccessorDescriptor(current) and ! IsAccessorDescriptor(Desc) are both true")
     ;;; b. If current.[[Configurable]] is false, then
     (if (is-false (:current -> Configurable))
         (;;; i. If Desc.[[Set]] is present and SameValue(Desc.[[Set]], current.[[Set]]) is false, return false.
          (if ((record-has-slot :Desc Set))
              ((if (is-false (call SameValue (:Desc -> Set) (:current -> Set)))
                   ((return false)))))
          ;;; ii. If Desc.[[Get]] is present and SameValue(Desc.[[Get]], current.[[Get]]) is false, return false.
          (if ((record-has-slot :Desc Get))
              ((if (is-false (call SameValue (:Desc -> Get) (:current -> Get)))
                   ((return false)))))
          ;;; iii. Return true.
          (return true)))))
   ;;; 9. If O is not undefined, then
   (if (isnt-undef :O)
       (;;; a. For each field of Desc that is present, set the corresponding attribute of the property named P of object O
        ;;;    to the value of the field.
        (P = (:O => :P))
        (record-copy-slot-if-present :Desc :P Value)
        (record-copy-slot-if-present :Desc :P Writable)
        (record-copy-slot-if-present :Desc :P Get)
        (record-copy-slot-if-present :Desc :P Set)
        (record-copy-slot-if-present :Desc :P Configurable)
        (record-copy-slot-if-present :Desc :P Enumerable)))
   ;;; 10. Return true
   (return true)))

(section
  (:10.1.1 OrdinaryObjectInternalMethods_GetPrototypeOf (O))
  (;;; 1. Return ! OrdinaryGetPrototypeOf(O).
   (return (! (call OrdinaryGetPrototypeOf :O)))))

(section
  (:10.1.1.1 OrdinaryGetPrototypeOf (O))
  (;;; 1. Return O.[[Prototype]].
   (return (:O -> Prototype))))

(section
  (:10.1.3 OrdinaryObjectInternalMethods_IsExtensible (O))
  (;;; 1. Return ! OrdinaryIsExtensible(O).
   (return (! (call OrdinaryIsExtensible :O)))))

(section
  (:10.1.3.1 OrdinaryIsExtensible (O))
  (;;; 1. Return O.[[Extensible]].
   (return (:O -> Extensible))))

(section
  (:10.1.5 OrdinaryObjectInternalMethods_GetOwnProperty (O, P))
  (;;; 1. Return ! OrdinaryGetOwnProperty(O, P).
   (return (! (call OrdinaryGetOwnProperty :O :P)))))

(section
  (:10.1.5.1 OrdinaryGetOwnProperty (O, P))
  (;;; 1. If O does not have an own property with key P, return undefined.
   (if (record-absent-prop :O :P)
       ((return undefined)))
   ;;; 2. Let D be a newly created Property Descriptor with no fields.
   (D = record-new)
   ;;; 3. Let X be O's own property whose key is P.
   (X = (:O => :P))
   ;;; 4. If X is a data property, then
   (if (call IsDataDescriptor :X)
       (;;; a. Set D.[[Value]] to the value of X's [[Value]] attribute.
        (:D Value <- (:X -> Value))
        ;;; b. Set D.[[Writable]] to the value of X's [[Writable]] attribute.
        (:D Writable <- (:X -> Writable)))
       ;;; 5. Else,
       (;;; a. Assert: X is an accessor property.
        (assert (call IsAccessorDescriptor :X) "X is an accessor property.")
        ;;; b. Set D.[[Get]] to the value of X's [[Get]] attribute.
        (:D Get <- (:X -> Get))
        ;;; c. Set D.[[Set]] to the value of X's [[Set]] attribute.
        (:D Set <- (:X -> Set))))
   ;;; 6. Set D.[[Enumerable]] to the value of X's [[Enumerable]] attribute.
   (:D Enumerable <- (:X -> Enumerable))
   ;;; 7. Set D.[[Configurable]] to the value of X's [[Configurable]] attribute.
   (:D Configurable <- (:X -> Configurable))
   ;;; 8. Return D.
   (return :D)))

(section
  (:10.1.6 OrdinaryObjectInternalMethods_DefineOwnProperty (O, P, Desc))
  (;;; 1. Return ? OrdinaryDefineOwnProperty(O, P, Desc).
   (return (? (call OrdinaryDefineOwnProperty :O :P :Desc)))))

(section
  (:10.1.6.1 OrdinaryDefineOwnProperty (O, P, Desc))
  (;;; 1. Let current be ? O.[[GetOwnProperty]](P).
   (current = (? (:O .. GetOwnProperty :P)))
   ;;; 2. Let extensible be ? IsExtensible(O).
   (extensible = (? (call IsExtensible :O)))
   ;;; 3. Return ValidateAndApplyPropertyDescriptor(O, P, extensible, Desc, current).
   (return (call ValidateAndApplyPropertyDescriptor :O :P :extensible :Desc :current))))

(section
  (:10.1.7 OrdinaryObjectInternalMethods_HasProperty (O, P))
  (;;; 1. Return ? OrdinaryHasProperty(O, P).
   (return (? (call OrdinaryHasProperty :O :P)))))

(section
  (:10.1.7.1 OrdinaryHasProperty (O, P))
  (;;; 1. Let hasOwn be ? O.[[GetOwnProperty]](P).
   (hasOwn = (? (:O .. GetOwnProperty :P)))
   ;;; 2. If hasOwn is not undefined, return true.
   (if (isnt-undef :hasOwn)
       ((return true)))
   ;;; 3. Let parent be ? O.[[GetPrototypeOf]]().
   (parent = (? (:O .. GetPrototypeOf)))
   ;;; 4. If parent is not null, then
   (if (isnt-null :parent)
       (;;; a. Return ? parent.[[HasProperty]](P).
        (return (? (:parent .. HasProperty :P)))))
   ;;; 5. Return false.
   (return false)))

(section
  (:10.1.12 OrdinaryObjectCreate (proto, additionalInternalSlotsList))
  (;;; 1. Let internalSlotsList be « [[Prototype]], [[Extensible]] ».
   (internalSlotsList = (list-new-2 (trivial-slot Prototype) (trivial-slot Extensible)))
   ;;; 2. If additionalInternalSlotsList is present, append each of its elements to internalSlotsList.
   (for :additionalInternalSlotsList
        ((list-push :internalSlotsList for-item)))
   ;;; 3. Let O be ! MakeBasicObject(internalSlotsList).
   (O = (! (call MakeBasicObject :internalSlotsList)))
   ;;; 4. Set O.[[Prototype]] to proto.
   (:O Prototype <- :proto)
   ;;; 5. Return O.
   (return :O)))

(section
  (:10.2.1 FunctionObject_Call (F, thisArgument, argumentsList))
  (;;; 1. Let callerContext be the running execution context.
   (callerContext = curr-exec-ctx)
   ;;; 2. Let calleeContext be PrepareForOrdinaryCall(F, undefined).
   (calleeContext = (call PrepareForOrdinaryCall :F undefined))
   ;;; 3. Assert: calleeContext is now the running execution context.
   (assert (:calleeContext == curr-exec-ctx) "calleeContext is now the running execution context.")
   ;;; 4. If F.[[IsClassConstructor]] is true, then
   (if (is-true (:F -> IsClassConstructor))
       (;;; a. Let error be a newly created TypeError object.
        ;;; b. NOTE: error is created in calleeContext with F's associated Realm Record.
        (error = (TypeError "function is class constructor"))
        ;;; c. Remove calleeContext from the execution context stack and restore callerContext as the running execution context.
        (exec-ctx-stack-pop)
        (exec-ctx-stack-push :callerContext)
        ;;; d. Return ThrowCompletion(error).
        (return (ThrowCompletion :error))))
   ;;; 5. Perform OrdinaryCallBindThis(F, calleeContext, thisArgument).
   (call OrdinaryCallBindThis :F :calleeContext :thisArgument)
   ;;; 6. Let result be OrdinaryCallEvaluateBody(F, argumentsList).
   (result = (call OrdinaryCallEvaluateBody :F :argumentsList))
   ;;; 7. Remove calleeContext from the execution context stack and restore callerContext as the running execution context.
   (exec-ctx-stack-pop)
   (exec-ctx-stack-push :callerContext)
   ;;; 8. If result.[[Type]] is return, return NormalCompletion(result.[[Value]]).
   (if ((:result -> Type) == trivial-return)
       ((return (NormalCompletion (:result -> Value)))))
   ;;; 9. ReturnIfAbrupt(result).
   (dontCare = (? :result))
   ;;; 10. Return NormalCompletion(undefined).
   (return (NormalCompletion undefined))))

(section
  (:10.2.1.1 PrepareForOrdinaryCall (F, newTarget))
  (;;; 1. Let callerContext be the running execution context.
   (callerContext = curr-exec-ctx)
   ;;; 2. Let calleeContext be a new ECMAScript code execution context.
   (calleeContext = record-new)
   ;;; 3. Set the Function of calleeContext to F.
   (:calleeContext Function <- :F)
   ;;; 4. Let calleeRealm be F.[[Realm]].
   (calleeRealm = (:F -> Realm))
   ;;; 5. Set the Realm of calleeContext to calleeRealm.
   (:calleeContext Realm <- :calleeRealm)
   ;;; 6. Set the ScriptOrModule of calleeContext to F.[[ScriptOrModule]].
   (:calleeContext ScriptOrModule <- (:F -> ScriptOrModule))
   ;;; 7. Let localEnv be NewFunctionEnvironment(F, newTarget).
   (localEnv = (call NewFunctionEnvironment :F :newTarget))
   ;;; 8. Set the LexicalEnvironment of calleeContext to localEnv.
   (:calleeContext LexicalEnvironment <- :localEnv)
   ;;; 9. Set the VariableEnvironment of calleeContext to localEnv.
   (:calleeContext VariableEnvironment <- :localEnv)
   ;;; 10. Set the PrivateEnvironment of calleeContext to F.[[PrivateEnvironment]].
   (:calleeContext PrivateEnvironment <- (:F -> PrivateEnvironment))
   ;;; 11. If callerContext is not already suspended, suspend callerContext.
   (exec-ctx-stack-pop)
   ;;; 12. Push calleeContext onto the execution context stack; calleeContext is now the running execution context.
   (exec-ctx-stack-push :calleeContext)
   ;;; 13. NOTE: Any exception objects produced after this point are associated with calleeRealm.
   ;;; 14. Return calleeContext.
   (return :calleeContext)))

(section
  (:10.2.1.2 OrdinaryCallBindThis (F, calleeContext, thisArgument))
  (;;; 1. Let thisMode be F.[[ThisMode]].
   (thisMode = (:F -> ThisMode))
   ;;; 2. If thisMode is lexical, return NormalCompletion(undefined).
   (if (:thisMode == lexical)
       ((return (NormalCompletion undefined))))
   ;;; 3. Let calleeRealm be F.[[Realm]].
   (calleeRealm = (:F -> Realm))
   ;;; 4. Let localEnv be the LexicalEnvironment of calleeContext.
   (localEnv = (:calleeContext -> LexicalEnvironment))
   ;;; 5. If thisMode is strict, let thisValue be thisArgument.
   (thisValue =
              (expr-block
               ((if (:thisMode == trivial-strict)
                    ((:thisArgument))
                    ;;; 6. Else,
                    (;;; a. If thisArgument is undefined or null, then
                     (if ((is-undef :thisArgument) or (is-null :thisArgument))
                         (;;; i. Let globalEnv be calleeRealm.[[GlobalEnv]].
                          (globalEnv = (:calleeRealm -> GlobalEnv))
                          ;;; ii. Assert: globalEnv is a global Environment Record.
                          ;;; iii. Let thisValue be globalEnv.[[GlobalThisValue]].
                          (:globalEnv -> GlobalThisValue))
                         ;;; b. Else,
                         (;;; i. Let thisValue be ! ToObject(thisArgument).
                          ;;; ii. NOTE: ToObject produces wrapper objects using calleeRealm.
                          (! (call ToObject :thisArgument)))))))))
   ;;; 7. Assert: localEnv is a function Environment Record.
   ;;; 8. Assert: The next step never returns an abrupt completion because localEnv.[[ThisBindingStatus]] is not initialized.
   ;;; 9. Return localEnv.BindThisValue(thisValue).
   (return (call BindThisValue :localEnv :thisValue))))

(section
  (:10.2.1.3 EvaluateBody (parseNode, F, argumentsList))
  ((todo)
   (return unreachable)))

(section
  (:10.2.1.4 OrdinaryCallEvaluateBody (F, argumentsList))
  (;;; 1. Return the result of EvaluateBody of the parsed code that is F.[[ECMAScriptCode]] passing F and argumentsList
   ;;;    as the arguments.
   (return (call EvaluateBody (:F -> ECMAScriptCode) :F :argumentsList))))

(section
  (:10.2.3 OrdinaryFunctionCreate (functionPrototype, sourceText, ParameterList, Body, thisMode, Scope, PrivateScope))
  (;;; 1. Let internalSlotsList be the internal slots listed in Table 34.
   (internalSlotsList = list-new)
   (inject-table-34 :internalSlotsList)
   ;;; 2. Let F be ! OrdinaryObjectCreate(functionPrototype, internalSlotsList).
   (F = (call OrdinaryObjectCreate :functionPrototype :internalSlotsList))
   ;;; 3. Set F.[[Call]] to the definition specified in 10.2.1.
   (:F Call <- (get-fn-ptr FunctionObject_Call))
   ;;; 4. Set F.[[SourceText]] to sourceText.
   (:F SourceText <- :sourceText)
   ;;; 5. Set F.[[FormalParameters]] to ParameterList.
   (:F FormalParameters <- :ParameterList)
   ;;; 6. Set F.[[ECMAScriptCode]] to Body.
   (:F ECMAScriptCode <- :Body)
   ;;; 7. If the source text matched by Body is strict mode code, let Strict be true; else let Strict be false.
   (Strict = true) ; TODO: strict mode stuff
   ;;; 8. Set F.[[Strict]] to Strict.
   (:F Strict <- :Strict)
   ;;; 9. If thisMode is lexical-this, set F.[[ThisMode]] to lexical.
   (if (:thisMode == lexical-this)
       ((:F ThisMode <- lexical))
       ;;; 10. Else if Strict is true, set F.[[ThisMode]] to strict.
       (elif (is-true :Strict)
             ((:F ThisMode <- trivial-strict))
             (;;; 11. Else, set F.[[ThisMode]] to global.
              (:F ThisMode <- trivial-global))))
   ;;; 12. Set F.[[IsClassConstructor]] to false.
   (:F IsClassConstructor <- false)
   ;;; 13. Set F.[[Environment]] to Scope.
   (:F Environment <- :Scope)
   ;;; 14. Set F.[[PrivateEnvironment]] to PrivateScope.
   (:F PrivateEnvironment <- :PrivateScope)
   ;;; 15. Set F.[[ScriptOrModule]] to GetActiveScriptOrModule().
   (:F ScriptOrModule <- (call GetActiveScriptOrModule))
   ;;; 16. Set F.[[Realm]] to the current Realm Record.
   (:F Realm <- current-realm)
   ;;; 17. Set F.[[HomeObject]] to undefined.
   (:F HomeObject <- undefined)
   ;;; 18. Set F.[[Fields]] to a new empty List.
   (:F Fields <- list-new)
   ;;; 19. Set F.[[PrivateMethods]] to a new empty List.
   (:F PrivateMethods <- list-new)
   ;;; 20. Set F.[[ClassFieldInitializerName]] to empty.
   (:F ClassFieldInitializerName <- empty)
   ;;; 21. Let len be the ExpectedArgumentCount of ParameterList.
   (len = (call ExpectedArgumentCount :ParameterList))
   ;;; 22. Perform ! SetFunctionLength(F, len).
   (tmp = (call SetFunctionLength :F :len))
   (Perform ! :tmp)
   ;;; 23. Return F.
   (return :F)))

(section
  (:10.2.9 SetFunctionName (F, name, prefix))
  (;;; 1. Assert: F is an extensible object that does not have a "name" own property.
   ;;; 2. If Type(name) is Symbol, then
   (name =
         (expr-block
          ((if (is-symbol :name)
               (;;; a. Let description be name's [[Description]] value.
                (description = (:name -> Description))
                ;;; b. If description is undefined, set name to the empty String.
                (if (is-undef :description)
                    ("")
                    ;;; c. Else, set name to the string-concatenation of "[", description, and "]".
                    (; TODO: string concat
                     :description)))
               ;;; 3. Else if name is a Private Name, then
               (elif (false)
                     (;;; a. Set name to name.[[Description]].
                      (:name -> Description))
                     (:name))))))
   ;;; 4. If F has an [[InitialName]] internal slot, then
   (if (record-has-slot :F InitialName)
       (;;; a. Set F.[[InitialName]] to name.
        (:F InitialName <- :name)))
   ;;; 5. If prefix is present, then
   (name =
         (expr-block
          ((if (isnt-undef :prefix)
               (;;; a. Set name to the string-concatenation of prefix, the code unit 0x0020 (SPACE), and name.
                ; TODO: string concatenation
                (nameTemp = :name)
                ;;; b. If F has an [[InitialName]] internal slot, then
                (if (record-has-slot :F InitialName)
                    (;;; i. Optionally, set F.[[InitialName]] to name.
                     (:F InitialName <- :nameTemp)))
                (:nameTemp))
               (:name)))))
   ;;; 6. Return ! DefinePropertyOrThrow(F, "name", PropertyDescriptor { [[Value]]: name, [[Writable]]: false, [[Enumerable]]: false,
   ;;;                                              [[Configurable]]: true }).
   (propDesc = record-new)
   (:propDesc Value <- :name)
   (:propDesc Writable <- false)
   (:propDesc Enumerable <- false)
   (:propDesc Configurable <- true)
   (return (! (call DefinePropertyOrThrow :F "name" :propDesc)))))

(section
  (:10.2.10 SetFunctionLength (F, length))
  (;;; 1. Assert: F is an extensible object that does not have a "length" own property.
   ;;; 2. Return ! DefinePropertyOrThrow(F, "length", PropertyDescriptor { [[Value]]: 𝔽(length), [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }).
   (propDesc = record-new)
   (:propDesc Value <- :length)
   (:propDesc Writable <- false)
   (:propDesc Enumerable <- false)
   (:propDesc Configurable <- true)
   (return (! (call DefinePropertyOrThrow :F "length" :propDesc)))))

(section
  (:10.3.3 CreateBuiltinFunction (behaviour, length, name, additionalInternalSlotsList, realm, prototype, prefix))
  (;;; 1. If realm is not present, set realm to the current Realm Record.
   (realm = (expr-block
             ((if (is-undef :realm)
                  ((curr-exec-ctx -> Realm))
                  (:realm)))))
   ;;; 2. If prototype is not present, set prototype to realm.[[Intrinsics]].[[%Function.prototype%]].
   (prototype = (expr-block
                 ((if (is-undef :prototype)
                      (record-new)
                      ; TODO
                      (:prototype)))))
   ;;; 3. Let internalSlotsList be a List containing the names of all the internal slots that 10.3 requires for the built-in
   ;;;    function object that is about to be created.
   (internalSlotsList = list-new)
   (list-push :internalSlotsList (trivial-slot Prototype))
   (list-push :internalSlotsList (trivial-slot Extensible))
   (inject-table-34 :internalSlotsList)
   (list-push :internalSlotsList (trivial-slot InitialName))
   ;;; 4. Append to internalSlotsList the elements of additionalInternalSlotsList.
   (for :additionalInternalSlotsList
        ((list-push :internalSlotsList for-item)))
   ;;; 5. Let func be a new built-in function object that, when called, performs the action described by behaviour using
   ;;;    the provided arguments as the values of the corresponding parameters specified by behaviour. The new function
   ;;;    object has internal slots whose names are the elements of internalSlotsList, and an [[InitialName]] internal
   ;;;    slot.
   ; (! (MakeBasicObject)) determined to be suitable here
   ; because engine262 does this
   (func = (! (call MakeBasicObject :internalSlotsList)))
   (:func InitialName <- undefined) ; this must be present i guess
   (:func Call <- :behaviour)
   ;;; 6. Set func.[[Prototype]] to prototype.
   (:func Prototype <- :prototype)
   ;;; 7. Set func.[[Extensible]] to true.
   (:func Extensible <- true)
   ;;; 8. Set func.[[Realm]] to realm.
   (:func Realm <- :realm)
   ;;; 9. Set func.[[InitialName]] to null.
   (:func InitialName <- null)
   ;;; 10. Perform ! SetFunctionLength(func, length).
   (Perform ! (call SetFunctionLength :func :length))
   ;;; 11. If prefix is not present, then
   (if (is-undef :prefix)
       (;;; a. Perform ! SetFunctionName(func, name).
        (Perform ! (call SetFunctionName :func :name undefined)))
       ;;; 12. Else,
       (;;; a. Perform ! SetFunctionName(func, name, prefix).
        (Perform ! (call SetFunctionName :func :name :prefix))))
   ;;; 13. Return func.
   (return :func)))

(section
  (:13.1.3 Evaluation_IdentifierReference (parseNode))
  (; IdentifierReference : Identifier
   (if (is-pn IdentifierReference 0)
       (;;; 1. Return ? ResolveBinding(StringValue of Identifier).
        (ret-comp (? (call ResolveBinding (:parseNode -> JSSATParseNodeSlot1 -> JSSATParseNode_Identifier_StringValue) undefined)))))
   ; IdentifierReference : yield
   (if (is-pn IdentifierReference 1)
       (;;; 1. Return ? ResolveBinding("yield").
        (ret-comp (? (call ResolveBinding "yield" undefined)))))
   ; IdentifierReference : await
   (if (is-pn IdentifierReference 2)
       (;;; 1. Return ? ResolveBinding("await").
        (ret-comp (? (call ResolveBinding "await" undefined)))))
   (return unreachable)))

(section
  (:15.1.4 HasInitializer (parseNode))
  (; BindingElement : BindingPattern
   (if (is-pn BindingElement 1)
       (;;; 1. Return false.
        (return false)))
   ; BindingElement : BindingPattern Initializer
   (if (is-pn BindingElement 2)
       (;;; 1. Return true.
        (return true)))
   ; SingleNameBinding : BindingIdentifier
   (if (is-pn SingleNameBinding 0)
       (;;; 1. Return false.
        (return false)))
   ; SingleNameBinding : BindingIdentifier Initializer
   (if (is-pn SingleNameBinding 1)
       (;;; 1. Return true.
        (return true)))
   ; FormalParameterList : FormalParameterList , FormalParameter
   (if (is-pn FormalParameterList 1)
       (;;; 1. If HasInitializer of FormalParameterList is true, return true.
        (if (is-true (call HasInitializer (:parseNode -> JSSATParseNodeSlot1)))
            ((return true)))
        ;;; 2. Return HasInitializer of FormalParameter.
        (return (call HasInitializer (:parseNode -> JSSATParseNodeSlot2)))))
   (return unreachable)))

(section
  (:15.1.5 ExpectedArgumentCount (parseNode))
  (; FormalParameters :
   ;     [empty]
   ;     FunctionRestParameter
   (if (or (is-pn FormalParameters 0) (is-pn FormalParameters 1))
       (;;; 1. Return 0.
        (return 0)))
   ; FormalParameters : FormalParameterList , FunctionRestParameter
   (if (is-pn FormalParameters 4)
       (;;; 1. Return ExpectedArgumentCount of FormalParameterList.
        (return (call ExpectedArgumentCount (:parseNode -> JSSATParseNodeSlot1)))))
   ; FormalParameterList : FormalParameter
   (if (is-pn FormalParameterList 0)
       (;;; 1. If HasInitializer of FormalParameter is true, return 0.
        (if (is-true (call HasInitializer (:parseNode -> JSSATParseNodeSlot1)))
            ((return 0)))
        ;;; 2. Return 1.
        (return 1)))
   ; FormalParameterList : FormalParameterList , FormalParameter
   (if (is-pn FormalParameterList 1)
       (;;; 1. Let count be ExpectedArgumentCount of FormalParameterList.
        (count = (call ExpectedArgumentCount (:parseNode -> JSSATParseNodeSlot1)))
        ;;; 2. If HasInitializer of FormalParameterList is true or HasInitializer of FormalParameter is true, return count.
        (if (or
             (is-true (call HasInitializer (:parseNode -> JSSATParseNodeSlot1)))
             (is-true (call HasInitializer (:parseNode -> JSSATParseNodeSlot2))))
            ((return :count)))
        ;;; 3. Return count + 1.
        (return (:count + 1))))
   ; ArrowParameters : BindingIdentifier
   (if (is-pn ArrowParameters 0)
       (;;; 1. Return 1.
        (return 1)))
   ; ArrowParameters : CoverParenthesizedExpressionAndArrowParameterList
   (if (is-pn ArrowParameters 1)
       (;;; 1. Let formals be the ArrowFormalParameters that is covered by CoverParenthesizedExpressionAndArrowParameterList.
        (todo)
        (assert false "in the future we'll have every parse node that's a `Cover X` automatically try to be parsed as a")
        (assert false "cover thing, so that we can just perform a member lookup")
        ;;; 2. Return ExpectedArgumentCount of formals.
       ))
   ; PropertySetParameterList : FormalParameter
   (if (is-pn PropertySetParameterList 0)
       (;;; 1. If HasInitializer of FormalParameter is true, return 0.
        (if (is-true (call HasInitializer (:parseNode -> JSSATParseNodeSlot1)))
            ((return 0)))
        ;;; 2. Return 1.
        (return 1)))
   ; AsyncArrowBindingIdentifier : BindingIdentifier
   (if (is-pn AsyncArrowBindingIdentifier 0)
       (;;; 1. Return 1.
        (return 1)))
   (return unreachable)))

(section
  (:16.1.6 ScriptEvaluation (scriptRecord))
  (;;; 1. Let globalEnv be scriptRecord.[[Realm]].[[GlobalEnv]].
   (globalEnv = ((:scriptRecord -> Realm) -> GlobalEnv))
   ;;; 2. Let scriptContext be a new ECMAScript code execution context.
   (scriptContext = record-new)
   ;;; 3. Set the Function of scriptContext to null.
   (:scriptContext Function <- null)
   ;;; 4. Set the Realm of scriptContext to scriptRecord.[[Realm]].
   (:scriptContext Realm <- (:scriptRecord -> Realm))
   ;;; 5. Set the ScriptOrModule of scriptContext to scriptRecord.
   (:scriptContext ScriptOrModule <- :scriptRecord)
   ;;; 6. Set the VariableEnvironment of scriptContext to globalEnv.
   (:scriptContext VariableEnvironment <- :globalEnv)
   ;;; 7. Set the LexicalEnvironment of scriptContext to globalEnv.
   (:scriptContext LexicalEnvironment <- :globalEnv)
   ;;; 8. Suspend the currently running execution context.
   (exec-ctx-stack-pop)
   ;;; 9. Push scriptContext onto the execution context stack; scriptContext is now the running execution context.
   (exec-ctx-stack-push :scriptContext)
   ;;; 10. Let scriptBody be scriptRecord.[[ECMAScriptCode]].
   (scriptBody = (:scriptRecord -> ECMAScriptCode))
   ;;; 11. Let result be GlobalDeclarationInstantiation(scriptBody, globalEnv).
   (result = (call GlobalDeclarationInstantiation :scriptBody :globalEnv))
   ;;; 12. If result.[[Type]] is normal, then
   (result = (if (is-normal (:result -> Type))
                 (;;; a. Set result to the result of evaluating scriptBody.
                  (evaluating :scriptBody))
                 (:result)))
   ;;; 13. If result.[[Type]] is normal and result.[[Value]] is empty, then
   (result = (if ((is-normal (:result -> Type)) and (is-empty (:result -> Value)))
                 (;;; a. Set result to NormalCompletion(undefined).
                  (NormalCompletion undefined))
                 (:result)))
   ;;; 14. Suspend scriptContext and remove it from the execution context stack.
   ; (todo)
   ;;; 15. Assert: The execution context stack is not empty.
   (assert (0 != exec-ctx-stack-size) "The execution context stack is not empty.")
   ;;; 16. Resume the context that is now on the top of the execution context stack as the running execution context.
   ; (todo)
   ;;; 17. Return Completion(result).
   (return :result)))

(section
  (:16.1.5 ParseScript (sourceText, realm, hostDefined, body))
  (;;; 1. Let body be ParseText(sourceText, Script).
   ; we have already parsed the script
   ;;; 2. If body is a List of errors, return body.
   ;;; 3. Return Script Record { [[Realm]]: realm, [[ECMAScriptCode]]: body, [[HostDefined]]: hostDefined }.
   (scriptRecord = record-new)
   (:scriptRecord Realm <- :realm)
   (:scriptRecord ECMAScriptCode <- :body)
   (:scriptRecord HostDefined <- :hostDefined)
   (return :scriptRecord)))

(section
  (:16.1.7 GlobalDeclarationInstantiation (script, env))
  (;;; 1. Assert: env is a global Environment Record.
   ; (todo)
   ;;; 2. Let lexNames be the LexicallyDeclaredNames of script.
   (lexNames = (call LexicallyDeclaredNames :script))
   ;;; 3. Let varNames be the VarDeclaredNames of script.
   (varNames = (call VarDeclaredNames :script))
   ;;; 4. For each element name of lexNames, do
   (for :varNames
        ((name = for-item)
         ;;; a. If env.HasVarDeclaration(name) is true, throw a SyntaxError exception.
         (if (:env .. HasVarDeclaration :name)
             ((throw (SyntaxError :env "env.HasVarDeclaration(name) is true"))))
         ;;; b. If env.HasLexicalDeclaration(name) is true, throw a SyntaxError exception.
         (if (:env .. HasLexicalDeclaration :name)
             ((throw (SyntaxError :env "env.HasLexicalDeclaration(name) is true"))))
         ;;; c. Let hasRestrictedGlobal be ? env.HasRestrictedGlobalProperty(name).
         (hasRestrictedGlobal = (? (:env .. HasRestrictedGlobalProperty :name)))
         ;;; d. If hasRestrictedGlobal is true, throw a SyntaxError exception.
         (if (is-true :hasRestrictedGlobal)
             ((throw (SyntaxError :env "If hasRestrictedGlobal is true"))))))
   ;;; 5. For each element name of varNames, do
   (for :varNames
        ((name = for-item)
         ;;; a. If env.HasLexicalDeclaration(name) is true, throw a SyntaxError exception.
         (hasLexicalDeclaration = (:env .. HasLexicalDeclaration :name))
         (if (is-true :hasLexicalDeclaration)
             ((throw (SyntaxError :env "If env.HasLexicalDeclaration(name) is true"))))))
   ;;; 6. Let varDeclarations be the VarScopedDeclarations of script.
   (varDeclarations = (call VarScopedDeclarations :script))
   ;;; 7. Let functionsToInitialize be a new empty List.
   (functionsToInitialize = list-new)
   ;;; 8. Let declaredFunctionNames be a new empty List.
   (declaredFunctionNames = list-new)
   ;;; 9. For each element d of varDeclarations, in reverse List order, do
   (for :varDeclarations
        ((d = for-item-rev)
         ;;; a. If d is neither a VariableDeclaration nor a ForBinding nor a BindingIdentifier, then
         ;;; i. Assert: d is either a FunctionDeclaration, a GeneratorDeclaration, an AsyncFunctionDeclaration, or an AsyncGeneratorDeclaration.
         ;;; ii. NOTE: If there are multiple function declarations for the same name, the last declaration is used.
         ;;; iii. Let fn be the sole element of the BoundNames of d.
         (fn = (sole-element (call BoundNames :d)))
         ;;; iv. If fn is not an element of declaredFunctionNames, then
         ;;; 1. Let fnDefinable be ? env.CanDeclareGlobalFunction(fn).
         ;;; 2. If fnDefinable is false, throw a TypeError exception.
         ;;; 3. Append fn to declaredFunctionNames.
         ;;; 4. Insert d as the first element of functionsToInitialize.
        ))
   ;;; 10. Let declaredVarNames be a new empty List.
   (declaredVarNames = list-new)
   ;;; 11. For each element d of varDeclarations, do
   (for :varDeclarations
        ((d = for-item)
         ;;; a. If d is a VariableDeclaration, a ForBinding, or a BindingIdentifier, then
         ;;; i. For each String vn of the BoundNames of d, do
         (boundNamesOfD = (call BoundNames :d))
         (for :boundNamesOfD
              ((vn = for-item)
               ;;; 1. If vn is not an element of declaredFunctionNames, then
               ;;; a. Let vnDefinable be ? env.CanDeclareGlobalVar(vn).
               ;;; b. If vnDefinable is false, throw a TypeError exception.
               ;;; c. If vn is not an element of declaredVarNames, then
               ;;; i. Append vn to declaredVarNames.
              ))))
   ;;; 12. NOTE: No abnormal terminations occur after this algorithm step if the global object is an ordinary object.However, if the global object is a Proxy exotic object it may exhibit behaviours that cause abnormalterminations in some of the following steps.
   ;;; 13. NOTE: Annex B.3.3.2 adds additional steps at this point.
   ;;; 14. Let lexDeclarations be the LexicallyScopedDeclarations of script.
   (lexDeclarations = (call LexicallyScopedDeclarations :script))
   ;;; 15. For each element d of lexDeclarations, do
   (for :lexDeclarations
        ((d = for-item)
         ;;; a. NOTE: Lexically declared names are only instantiated here but not initialized.
         ;;; b. For each element dn of the BoundNames of d, do
         (boundNamesOfD = (call BoundNames :d))
         (for :boundNamesOfD
              ((dn = for-item)
               ;;; i. If IsConstantDeclaration of d is true, then
               ;;; 1. Perform ? env.CreateImmutableBinding(dn, true).
               ;;; ii. Else,
               ;;; 1. Perform ? env.CreateMutableBinding(dn, false).
              ))))
   ;;; 16. For each Parse Node f of functionsToInitialize, do
   (for :functionsToInitialize
        ((f = for-item)
         ;;; a. Let fn be the sole element of the BoundNames of f.
         (fn = (sole-element (call BoundNames :f)))
         ;;; b. Let fo be InstantiateFunctionObject of f with argument env.
         ;;; c. Perform ? env.CreateGlobalFunctionBinding(fn, fo, false).
        ))
   ;;; 17. For each String vn of declaredVarNames, do
   (for :declaredVarNames
        ((vn = for-item)
         ;;; a. Perform ? env.CreateGlobalVarBinding(vn, false).
        ))
   ;;; 18. Return NormalCompletion(empty).
   (return (NormalCompletion empty))))